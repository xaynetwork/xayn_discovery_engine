// Copyright 2021 Xayn AG
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

use anyhow::Error;
use derive_more::{Deref, From};
use displaydoc::Display;
use ndarray::{s, Array, Array1, Dimension, Ix, Ix1, Ix2};
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

use crate::{model::Prediction, tokenizer::AttentionMask};

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

/// Values don't represent a valid embedding.
#[derive(Clone, Debug, Display, Error, Serialize)]
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

/// An inert pooling strategy.
///
/// The prediction is just passed through.
pub struct NonePooler;

impl NonePooler {
    /// Passes through the prediction.
    pub(crate) fn pool(prediction: &Prediction) -> Result<Embedding2, Error> {
        Ok(prediction
            .to_array_view()?
            .slice(s![0, .., ..])
            .to_owned()
            .into())
    }
}

/// A first token pooling strategy.
///
/// The prediction is pooled over its first tokens (`[CLS]`).
pub struct FirstPooler;

impl FirstPooler {
    /// Pools the prediction over its first token.
    pub(crate) fn pool(prediction: &Prediction) -> Result<Embedding1, Error> {
        Ok(prediction
            .to_array_view()?
            .slice(s![0, 0, ..])
            .to_owned()
            .into())
    }
}

/// An average token pooling strategy.
///
/// The prediction is pooled over its averaged tokens.
pub struct AveragePooler;

impl AveragePooler {
    /// Pools the prediction over its averaged, active tokens.
    pub(crate) fn pool(
        prediction: &Prediction,
        attention_mask: &AttentionMask,
    ) -> Result<Embedding1, Error> {
        let attention_mask: Array1<f32> = attention_mask.slice(s![0, ..]).mapv(
            #[allow(clippy::cast_precision_loss)] // values are only 0 or 1
            |mask| mask as f32,
        );
        let count = attention_mask.sum();

        let average = if count > 0. {
            attention_mask.dot(&prediction.to_array_view()?.slice(s![0, .., ..])) / count
        } else {
            Array1::default(prediction.shape()[2])
        };

        Ok(average.into())
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::SQRT_2;

    use ndarray::{arr2, arr3};
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
    fn test_none() {
        let prediction = arr3(&[[[1., 2., 3.], [4., 5., 6.]]]).into();
        let embedding = NonePooler::pool(&prediction).unwrap();
        assert_approx_eq!(f32, embedding, [[1., 2., 3.], [4., 5., 6.]]);
    }

    #[test]
    fn test_first() {
        let prediction = arr3(&[[[1., 2., 3.], [4., 5., 6.]]]).into();
        let embedding = FirstPooler::pool(&prediction).unwrap();
        assert_approx_eq!(f32, embedding, [1., 2., 3.]);
    }

    #[test]
    fn test_average() {
        let prediction = arr3(&[[[1., 2., 3.], [4., 5., 6.]]]).into();

        let mask = arr2(&[[0, 0]]).into();
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [0., 0., 0.]);

        let mask = arr2(&[[0, 1]]).into();
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [4., 5., 6.]);

        let mask = arr2(&[[1, 0]]).into();
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [1., 2., 3.]);

        let mask = arr2(&[[1, 1]]).into();
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [2.5, 3.5, 4.5]);
    }
}
