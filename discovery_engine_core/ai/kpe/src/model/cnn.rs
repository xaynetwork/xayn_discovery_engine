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
use ndarray::{concatenate, s, Array2, Axis, ErrorKind, NewAxis, ShapeError};

use crate::{
    model::{
        bert::{Bert, Embeddings},
        ModelError,
    },
    tokenizer::encoding::ValidMask,
};
use xayn_discovery_engine_layer::{activation::Relu, conv::Conv1D, io::BinParams};

/// A CNN model.
#[derive(Debug)]
pub(crate) struct Cnn {
    layers: [Conv1D<Relu>; Self::KEY_PHRASE_SIZE],
}

/// The inferred features.
///
/// The features are of shape `(channel_out_size = 512, sum(sum(valid_ids) - kernel_size + 1))`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct Features(pub(crate) Array2<f32>);

impl Features {
    /// Checks if the features are valid, i.e. finite.
    pub(crate) fn is_valid(&self) -> bool {
        self.iter().copied().all(f32::is_finite)
    }
}

impl Cnn {
    /// The maximum number of words per key phrase.
    pub(crate) const KEY_PHRASE_SIZE: usize = 5;

    /// The number of channels going out of the CNN layers.
    pub(crate) const CHANNEL_OUT_SIZE: usize = 512;

    /// Creates a model from a binary parameters file.
    pub(crate) fn new(mut params: BinParams) -> Result<Self, ModelError> {
        let mut new_layer = |scope| Conv1D::load(params.with_scope(scope), Relu, 1, 0, 1, 1);
        let layers = [
            new_layer("conv_1")?,
            new_layer("conv_2")?,
            new_layer("conv_3")?,
            new_layer("conv_4")?,
            new_layer("conv_5")?,
        ];
        if !params.is_empty() {
            return Err(ModelError::UnusedParams(
                params.keys().map(Into::into).collect(),
            ));
        }

        if layers
            .iter()
            .zip(1..=Self::KEY_PHRASE_SIZE)
            .all(|(layer, kernel_size)| {
                layer.channel_out_size() == Self::CHANNEL_OUT_SIZE
                    && layer.channel_grouped_size() == Bert::EMBEDDING_SIZE
                    && layer.kernel_size() == kernel_size
            })
        {
            Ok(Self { layers })
        } else {
            Err(ShapeError::from_kind(ErrorKind::IncompatibleShape).into())
        }
    }

    /// Runs the model on the valid embeddings to compute the convolved features.
    pub(crate) fn run(
        &self,
        embeddings: &Embeddings,
        valid_mask: &ValidMask,
    ) -> Result<Features, ModelError> {
        if !valid_mask.is_valid(Self::KEY_PHRASE_SIZE) {
            return Err(ModelError::NotEnoughWords);
        }

        debug_assert_eq!(
            embeddings.shape(),
            [1, valid_mask.len(), Bert::EMBEDDING_SIZE],
        );
        debug_assert!(embeddings.is_valid());
        let valid_size = cfg!(debug_assertions)
            .then(|| valid_mask.count())
            .unwrap_or_default();
        let valid_embeddings = embeddings.collect(valid_mask)?;
        debug_assert_eq!(valid_embeddings.shape(), [valid_size, Bert::EMBEDDING_SIZE]);
        debug_assert!(valid_embeddings.iter().copied().all(f32::is_finite));

        let run_layer =
            |idx: usize| self.layers[idx].run(&valid_embeddings.t().slice(s![NewAxis, .., ..]));
        let features = Features(concatenate(
            Axis(1),
            &[
                run_layer(0)?.slice(s![0, .., ..]),
                run_layer(1)?.slice(s![0, .., ..]),
                run_layer(2)?.slice(s![0, .., ..]),
                run_layer(3)?.slice(s![0, .., ..]),
                run_layer(4)?.slice(s![0, .., ..]),
            ],
        )?);
        debug_assert_eq!(
            features.shape(),
            [
                Self::CHANNEL_OUT_SIZE,
                Self::output_size(valid_embeddings.shape()[0]),
            ],
        );
        debug_assert!(features.is_valid());

        Ok(features)
    }

    /// Computes the output size of the concatenated CNN layers.
    fn output_size(valid_size: usize) -> usize {
        debug_assert!(valid_size >= Self::KEY_PHRASE_SIZE);
        Self::KEY_PHRASE_SIZE * valid_size
            - (Self::KEY_PHRASE_SIZE * (Self::KEY_PHRASE_SIZE - 1)) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_shapes() {
        assert_eq!(Cnn::KEY_PHRASE_SIZE, 5);
        assert_eq!(Cnn::CHANNEL_OUT_SIZE, 512);
    }

    #[test]
    fn test_model_empty() {
        matches!(
            Cnn::new(BinParams::default()).unwrap_err(),
            ModelError::Cnn(_),
        );
    }
}
