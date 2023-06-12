// Copyright 2023 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::ops::{Add, Mul};

use derive_more::{Deref, From};
use displaydoc::Display;
use itertools::Itertools;
use ndarray::{Array, Array1, Dimension, Ix, Ix1, Ix2};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "sqlx")]
use sqlx::{
    database::{HasArguments, HasValueRef},
    encode::IsNull,
    error::BoxDynError,
    Database,
    Decode,
    Encode,
    FromRow,
    Postgres,
    Type,
};
use thiserror::Error;
use xayn_test_utils::ApproxEqIter;

/// A d-dimensional sequence embedding.
#[derive(Clone, Debug, Deref, From, Default)]
pub struct Embedding<D>(Array<f32, D>)
where
    D: Dimension;

impl<D> Add for Embedding<D>
where
    D: Dimension,
{
    type Output = Embedding<D>;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 += &rhs.0;
        self
    }
}

impl<'a, D> ApproxEqIter<'a, f32> for Embedding<D>
where
    D: 'a + Dimension,
{
    fn indexed_iter_logical_order(
        &'a self,
        index_prefix: Vec<Ix>,
    ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, f32)>> {
        (**self).indexed_iter_logical_order(index_prefix)
    }
}

/// A 1-dimensional sequence embedding.
///
/// The embedding is of shape `(embedding_size,)`. The serde is identical to a `Vec<f32>`.
pub type Embedding1 = Embedding<Ix1>;

/// A normalized embedding.
#[derive(Clone, Debug, Deref, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(FromRow, Type), sqlx(transparent))]
pub struct NormalizedEmbedding(Embedding1);

#[derive(Clone, Debug, Display, Error, Serialize)]
/// Values don't represent a valid embedding.
pub struct InvalidEmbedding;

impl Embedding1 {
    pub fn normalize(mut self) -> Result<NormalizedEmbedding, InvalidEmbedding> {
        let norm = self.dot(&*self).sqrt();
        if !norm.is_finite() {
            return Err(InvalidEmbedding);
        }

        if norm > 0. {
            self.0 /= norm;
        } else {
            self.0 = Array1::zeros(self.len());
        };

        Ok(NormalizedEmbedding(self))
    }
}

impl From<Vec<f32>> for Embedding1 {
    fn from(vec: Vec<f32>) -> Self {
        Array1::from_vec(vec).into()
    }
}

impl<const N: usize> From<[f32; N]> for Embedding1 {
    fn from(array: [f32; N]) -> Self {
        Vec::from(array).into()
    }
}

impl Serialize for Embedding1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(&self.0)
    }
}

impl<'de> Deserialize<'de> for Embedding1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<f32>::deserialize(deserializer).map(Self::from)
    }
}

#[cfg(feature = "sqlx")]
impl Type<Postgres> for Embedding1 {
    fn type_info() -> <Postgres as Database>::TypeInfo {
        Vec::<f32>::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'q> Encode<'q, Postgres> for Embedding1 {
    fn encode_by_ref(&self, buf: &mut <Postgres as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
        if let Some(embedding) = self.as_slice() {
            embedding.encode_by_ref(buf)
        } else {
            self.to_vec().encode_by_ref(buf)
        }
    }
}

#[cfg(feature = "sqlx")]
impl<'r> Decode<'r, Postgres> for Embedding1 {
    fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
        Vec::<f32>::decode(value).map(Into::into)
    }
}

impl NormalizedEmbedding {
    /// The value is bounded in `[-1, 1]`.
    pub fn dot_product(&self, other: &Self) -> f32 {
        self.dot(&other.0 .0).clamp(-1., 1.)
    }
}

impl TryFrom<Vec<f32>> for NormalizedEmbedding {
    type Error = InvalidEmbedding;

    fn try_from(vec: Vec<f32>) -> Result<Self, Self::Error> {
        Embedding1::from(vec).normalize()
    }
}

impl<const N: usize> TryFrom<[f32; N]> for NormalizedEmbedding {
    type Error = InvalidEmbedding;

    fn try_from(array: [f32; N]) -> Result<Self, Self::Error> {
        Embedding1::from(array).normalize()
    }
}

impl Mul<f32> for &NormalizedEmbedding {
    type Output = Embedding1;

    fn mul(self, rhs: f32) -> Self::Output {
        (&self.0 .0 * rhs).into()
    }
}

impl<'a> ApproxEqIter<'a, f32> for NormalizedEmbedding {
    fn indexed_iter_logical_order(
        &'a self,
        index_prefix: Vec<Ix>,
    ) -> Box<dyn 'a + Iterator<Item = (Vec<Ix>, f32)>> {
        (**self).indexed_iter_logical_order(index_prefix)
    }
}

/// A 2-dimensional sequence embedding.
///
/// The embedding is of shape `(token_size, embedding_size)`.
pub type Embedding2 = Embedding<Ix2>;

/// A sparse sequence embedding.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow, Type), sqlx(type_name = "RECORD"))]
pub struct SparseEmbedding {
    // signed indices because of database limitations, alternatively we could transmute a Vec<u32>
    // to a Vec<i32> before database operations but that would require unsafe code
    indices: Vec<i32>,
    values: Vec<f32>,
}

/// A normalized sparse sequence embedding.
#[derive(Clone, Debug, Default, Deref, Deserialize, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(FromRow, Type), sqlx(transparent))]
pub struct NormalizedSparseEmbedding(SparseEmbedding);

impl SparseEmbedding {
    pub fn new(indices: Vec<i32>, values: Vec<f32>) -> Result<Self, InvalidEmbedding> {
        if !indices.is_empty()
            && indices.len() <= 1_000
            && indices.len() == values.len()
            && indices.iter().all_unique()
            && indices.iter().all(|index| !index.is_negative())
            && values.iter().all(|value| *value != 0. && value.is_finite())
        {
            Ok(Self { indices, values })
        } else {
            Err(InvalidEmbedding)
        }
    }

    pub fn normalize(mut self) -> Result<NormalizedSparseEmbedding, InvalidEmbedding> {
        let norm = self
            .values
            .iter()
            .map(|value| value * value)
            .sum::<f32>()
            .sqrt();

        if norm.is_finite() {
            for value in &mut self.values {
                *value /= norm;
            }
            Ok(NormalizedSparseEmbedding(self))
        } else {
            Err(InvalidEmbedding)
        }
    }
}

impl NormalizedSparseEmbedding {
    pub fn indices(&self) -> &[i32] {
        &self.indices
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

impl Mul<f32> for &NormalizedSparseEmbedding {
    type Output = Result<SparseEmbedding, InvalidEmbedding>;

    fn mul(self, rhs: f32) -> Self::Output {
        SparseEmbedding::new(
            self.indices.clone(),
            self.values.iter().map(|value| value * rhs).collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_1_SQRT_2, SQRT_2};

    use xayn_test_utils::assert_approx_eq;

    use super::*;

    #[test]
    fn test_normalize() {
        assert!(Embedding1::from([f32::NAN]).normalize().is_err());
        assert!(Embedding1::from([f32::INFINITY]).normalize().is_err());
        assert!(Embedding1::from([f32::NEG_INFINITY]).normalize().is_err());

        let embedding = Embedding1::from([0., 0., 0.]);
        assert_approx_eq!(f32, embedding.clone().normalize().unwrap(), embedding);

        let embedding = Embedding1::from([0., 1., 2., 3., SQRT_2])
            .normalize()
            .unwrap();
        assert_approx_eq!(f32, embedding, [0., 0.25, 0.5, 0.75, SQRT_2.powi(-3)]);

        let embedding = Embedding1::from([-1., 1., -1., 1.]).normalize().unwrap();
        assert_approx_eq!(f32, embedding, [-0.5, 0.5, -0.5, 0.5]);
    }

    #[test]
    fn test_new_sparse_embedding() {
        assert!(SparseEmbedding::new(vec![], vec![]).is_err());
        assert!(SparseEmbedding::new(vec![0], vec![1., 1.]).is_err());
        assert!(SparseEmbedding::new(vec![0, 0], vec![1., 1.]).is_err());
        assert!(SparseEmbedding::new(vec![-1], vec![1.]).is_err());
        assert!(SparseEmbedding::new(vec![0], vec![0.]).is_err());
        assert!(SparseEmbedding::new(vec![0], vec![f32::NAN]).is_err());

        let embedding = SparseEmbedding::new(vec![0, 2], vec![1., -1.]).unwrap();
        assert_eq!(embedding.indices, [0, 2]);
        assert_approx_eq!(f32, embedding.values, [1., -1.]);
    }

    #[test]
    fn test_normalize_sparse_embedding() {
        assert!(SparseEmbedding::new(vec![0], vec![f32::MAX])
            .unwrap()
            .normalize()
            .is_err());

        let embedding = SparseEmbedding::new(vec![0, 2], vec![1., -1.])
            .unwrap()
            .normalize()
            .unwrap();
        assert_eq!(embedding.indices, [0, 2]);
        assert_approx_eq!(f32, embedding.values, [FRAC_1_SQRT_2, -FRAC_1_SQRT_2]);
    }
}
