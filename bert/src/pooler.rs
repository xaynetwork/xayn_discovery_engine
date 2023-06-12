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

use ndarray::{s, Array1};
use tract_onnx::prelude::TractError;

use crate::{
    embedding::{Embedding1, Embedding2},
    model::Prediction,
};

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
        attention_mask: &[u32],
    ) -> Result<Embedding1, TractError> {
        let attention_mask = Array1::from_shape_fn(
            attention_mask.len(),
            #[allow(clippy::cast_precision_loss)] // values are only 0 or 1
            |i| attention_mask[i] as f32,
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
    use ndarray::arr3;
    use xayn_test_utils::assert_approx_eq;

    use super::*;

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

        let mask = [0, 0];
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [0., 0., 0.]);

        let mask = [0, 1];
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [4., 5., 6.]);

        let mask = [1, 0];
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [1., 2., 3.]);

        let mask = [1, 1];
        let embedding = AveragePooler::pool(&prediction, &mask).unwrap();
        assert_approx_eq!(f32, embedding, [2.5, 3.5, 4.5]);
    }
}
