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

use std::{
    mem::size_of,
    ops::{AddAssign, Mul, MulAssign},
};

use derive_more::{Deref, From};
use displaydoc::Display;
use float_cmp::{ApproxEq, F32Margin};
use ndarray::{s, Array, Array1, ArrayBase, Data, Dimension, Ix1, Ix2, Zip};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use tract_onnx::prelude::TractError;

use crate::{model::Prediction, tokenizer::AttentionMask};

/// A d-dimensional sequence embedding.
#[derive(Clone, Debug, Deref, From, Default)]
pub struct Embedding<D>(Array<f32, D>)
where
    D: Dimension;

/// A 1-dimensional sequence embedding.
///
/// The embedding is of shape `(embedding_size,)`. The serde is identical to a `Vec<f32>`.
pub type Embedding1 = Embedding<Ix1>;

impl Embedding<Ix1> {
    /// Converts from values in logical order to bytes in little endianness.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.iter().flat_map(|value| value.to_le_bytes()).collect()
    }

    pub fn normalize(&self) -> Result<Self, InvalidVectorEncounteredError> {
        let view = self.view();
        let norm = view.dot(&view).sqrt();
        if norm.is_finite() {
            let normalized = if norm <= 0. {
                Array1::zeros(self.len())
            } else {
                &self.0 / norm
            };
            Ok(normalized.into())
        } else {
            Err(InvalidVectorEncounteredError)
        }
    }
}

impl<const N: usize> From<[f32; N]> for Embedding<Ix1> {
    fn from(array: [f32; N]) -> Self {
        Array::from_vec(array.into()).into()
    }
}

impl From<Vec<f32>> for Embedding<Ix1> {
    fn from(vec: Vec<f32>) -> Self {
        Array::from_vec(vec).into()
    }
}

impl Serialize for Embedding<Ix1> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(&self.0)
    }
}

impl<'de> Deserialize<'de> for Embedding<Ix1> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<f32>::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Clone, Debug, Display, Error)]
/// Bytes do not represent a valid embedding.
pub struct MalformedBytesEmbedding;

impl TryFrom<Vec<u8>> for Embedding<Ix1> {
    type Error = MalformedBytesEmbedding;

    /// Converts from bytes in little endianness to values in standard order.
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() % size_of::<f32>() != 0 {
            return Err(MalformedBytesEmbedding);
        }

        let floats = bytes
            .chunks_exact(size_of::<f32>())
            .map(|chunk| {
                f32::from_le_bytes(chunk.try_into().unwrap(/* checked length before */))
            })
            .collect();

        Ok(Array::from_vec(floats).into())
    }
}

/// Computes the l2 norm (euclidean metric).
///
/// *NOTE* This used to `panic` before, but as we now calculate l2 norm upon ingestion,
/// this behavior changed to become `None`, so that underlying code could still follow up.
fn l2_norm<S>(
    a: ArrayBase<S, Ix1>,
    b: ArrayBase<S, Ix1>,
) -> Result<f32, InvalidVectorEncounteredError>
where
    S: Data<Elem = f32>,
{
    match a.view().dot(&b.view()).sqrt() {
        n if n.is_finite() => Ok(n),
        _ => Err(InvalidVectorEncounteredError),
    }
}

/// A 2-dimensional sequence embedding.
///
/// The embedding is of shape `(token_size, embedding_size)`.
pub type Embedding2 = Embedding<Ix2>;

impl<S, D> PartialEq<ArrayBase<S, D>> for Embedding<D>
where
    S: Data<Elem = f32>,
    D: Dimension,
{
    fn eq(&self, other: &ArrayBase<S, D>) -> bool {
        if self.shape() != other.shape() {
            return false;
        }

        let margin = F32Margin::default();
        Zip::from(&self.0)
            .and(other)
            .all(|this, other| (*this).approx_eq(*other, margin))
    }
}

impl<S, D> PartialEq<Embedding<D>> for ArrayBase<S, D>
where
    S: Data<Elem = f32>,
    D: Dimension,
{
    fn eq(&self, other: &Embedding<D>) -> bool {
        other.eq(self)
    }
}

impl<D> PartialEq for Embedding<D>
where
    D: Dimension,
{
    fn eq(&self, other: &Self) -> bool {
        self.eq(&other.0)
    }
}

impl<D> AddAssign for Embedding<D>
where
    D: Dimension,
{
    fn add_assign(&mut self, rhs: Self) {
        self.0 += &rhs.0;
    }
}

impl<D> Mul<f32> for &Embedding<D>
where
    D: Dimension,
{
    type Output = Embedding<D>;

    fn mul(self, rhs: f32) -> Self::Output {
        (&self.0 * rhs).into()
    }
}

impl<D> MulAssign<f32> for Embedding<D>
where
    D: Dimension,
{
    fn mul_assign(&mut self, rhs: f32) {
        self.0 *= rhs;
    }
}

/// An inert pooling strategy.
///
/// The prediction is just passed through.
pub struct NonePooler;

impl NonePooler {
    /// Passes through the prediction.
    pub(crate) fn pool(prediction: &Prediction) -> Result<Embedding2, TractError> {
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
    pub(crate) fn pool(prediction: &Prediction) -> Result<Embedding1, TractError> {
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
    ) -> Result<Embedding1, TractError> {
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

#[derive(Clone, Debug, Display, Error)]
/// Bytes do not represent a valid embedding.
pub struct InvalidVectorEncounteredError;

#[cfg(test)]
mod tests {
    use ndarray::{arr1, arr2, arr3};
    use tract_onnx::prelude::IntoArcTensor;

    use super::*;

    #[test]
    fn test_none() {
        let prediction = arr3::<f32, _, _>(&[[[1., 2., 3.], [4., 5., 6.]]])
            .into_arc_tensor()
            .into();
        let embedding = NonePooler::pool(&prediction).unwrap();
        assert_eq!(embedding, arr2(&[[1., 2., 3.], [4., 5., 6.]]));
    }

    #[test]
    fn test_first() {
        let prediction = arr3::<f32, _, _>(&[[[1., 2., 3.], [4., 5., 6.]]])
            .into_arc_tensor()
            .into();
        let embedding = FirstPooler::pool(&prediction).unwrap();
        assert_eq!(embedding, arr1(&[1., 2., 3.]));
    }

    #[test]
    fn test_average() {
        let prediction = arr3::<f32, _, _>(&[[[1., 2., 3.], [4., 5., 6.]]]).into_arc_tensor();

        let mask = arr2(&[[0, 0]]).into();
        let embedding = AveragePooler::pool(&prediction.clone().into(), &mask).unwrap();
        assert_eq!(embedding, arr1(&[0., 0., 0.]));

        let mask = arr2(&[[0, 1]]).into();
        let embedding = AveragePooler::pool(&prediction.clone().into(), &mask).unwrap();
        assert_eq!(embedding, arr1(&[4., 5., 6.]));

        let mask = arr2(&[[1, 0]]).into();
        let embedding = AveragePooler::pool(&prediction.clone().into(), &mask).unwrap();
        assert_eq!(embedding, arr1(&[1., 2., 3.]));

        let mask = arr2(&[[1, 1]]).into();
        let embedding = AveragePooler::pool(&prediction.into(), &mask).unwrap();
        assert_eq!(embedding, arr1(&[2.5, 3.5, 4.5]));
    }

    #[test]
    fn test_l2_norm() {
        xayn_ai_test_utils::assert_approx_eq!(
            f32,
            l2_norm(arr1(&[1., 2., 3.]), arr1(&[1., 2., 3.])).unwrap(),
            3.741_657_5
        );
    }

    #[test]
    fn test_l2_norm_nan() {
        assert!(l2_norm(arr1(&[1., f32::NAN, 3.]), arr1(&[1., f32::NAN, 3.])).is_err());
    }

    #[test]
    fn test_l2_norm_inf() {
        assert!(l2_norm(
            arr1(&[1., f32::INFINITY, 3.]),
            arr1(&[1., f32::INFINITY, 3.])
        )
        .is_err());
    }

    #[test]
    fn test_l2_norm_neginf() {
        assert!(l2_norm(
            arr1(&[1., f32::NEG_INFINITY, 3.]),
            arr1(&[1., f32::NEG_INFINITY, 3.])
        )
        .is_err());
    }
}
