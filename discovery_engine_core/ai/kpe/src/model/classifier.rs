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

use derive_more::{Deref, From};

use ndarray::{ErrorKind, ShapeError};

use crate::{
    model::{
        cnn::{Cnn, Features},
        ModelError,
    },
    tokenizer::encoding::ActiveMask,
};
use xayn_discovery_engine_layer::{activation::Linear, dense::Dense, io::BinParams};

/// A Classifier model.
#[derive(Debug)]
pub(crate) struct Classifier {
    layer: Dense<Linear>,
}

/// The inferred scores.
///
/// The scores are of shape `(len(key_phrase_choices),)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct Scores(pub(crate) Vec<f32>);

impl Scores {
    /// Checks if the scores are valid, i.e. finite.
    pub(crate) fn is_valid(&self) -> bool {
        self.iter().copied().all(f32::is_finite)
    }
}

impl Classifier {
    /// Creates a model from a binary parameters file.
    pub(crate) fn new(mut params: BinParams) -> Result<Self, ModelError> {
        let layer = Dense::load(params.with_scope("dense"), Linear)?;
        if !params.is_empty() {
            return Err(ModelError::UnusedParams(
                params.keys().map(Into::into).collect(),
            ));
        }

        if layer.weights().shape() == [Cnn::CHANNEL_OUT_SIZE, 1] {
            Ok(Self { layer })
        } else {
            Err(ShapeError::from_kind(ErrorKind::IncompatibleShape).into())
        }
    }

    /// Runs the model on the convolved features to compute the scores.
    pub(crate) fn run(&self, features: &Features, active_mask: &ActiveMask) -> Scores {
        debug_assert_eq!(features.shape()[1], active_mask.shape()[1]);
        debug_assert!(features.is_valid());
        debug_assert!(active_mask.is_valid());
        let (scores, _) = self.layer.run(&features.t(), false);
        debug_assert_eq!(scores.shape(), [features.shape()[1], 1]);
        debug_assert!(scores.iter().copied().all(f32::is_finite));

        let scores = Scores(
            active_mask
                .rows()
                .into_iter()
                .map(|active| {
                    active
                    .iter()
                    .zip(scores.iter())
                    .filter_map(|(active, score)| active.then(|| score))
                    .copied()
                    .reduce(f32::max)
                    .unwrap(/* active mask must have entries in each row */)
                })
                .collect::<Vec<f32>>(),
        );
        debug_assert_eq!(scores.len(), active_mask.shape()[0]);
        debug_assert!(scores.is_valid());

        scores
    }
}

#[cfg(test)]
mod tests {
    use ndarray::Array2;

    use super::*;
    use xayn_discovery_engine_test_utils::kpe::classifier;

    #[test]
    fn test_model_empty() {
        matches!(
            Classifier::new(BinParams::default()).unwrap_err(),
            ModelError::Classifier(_),
        );
    }

    #[test]
    fn test_run_unique() {
        let output_size = 42;
        let model =
            Classifier::new(BinParams::deserialize_from_file(classifier().unwrap()).unwrap())
                .unwrap();
        let features = Array2::default((Cnn::CHANNEL_OUT_SIZE, output_size)).into();
        let active_mask = Array2::from_elem((output_size, output_size), true).into();
        assert_eq!(model.run(&features, &active_mask).len(), output_size);
    }

    #[test]
    fn test_run_duplicate() {
        let output_size = 42;
        let model =
            Classifier::new(BinParams::deserialize_from_file(classifier().unwrap()).unwrap())
                .unwrap();
        let features = Array2::default((Cnn::CHANNEL_OUT_SIZE, output_size)).into();
        let active_mask = Array2::from_elem((output_size / 2, output_size), true).into();
        assert_eq!(model.run(&features, &active_mask).len(), output_size / 2);
    }
}
