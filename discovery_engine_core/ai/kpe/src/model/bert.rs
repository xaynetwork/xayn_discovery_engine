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

use std::{ops::RangeInclusive, path::Path, sync::Arc, time::Instant};

use derive_more::{Deref, From};
use ndarray::{Array1, Array2, ArrayBase, Dim, ErrorKind, IxDynImpl, OwnedRepr, ShapeError};
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use tracing::info;

use crate::{
    model::ModelError,
    tokenizer::encoding::{AttentionMask, TokenIds, ValidMask},
};

/// A Bert onnx model.
#[derive(Debug)]
pub(crate) struct Bert {
    session: Session,
    token_size: usize,
}

/// The inferred embeddings.
///
/// The embeddings are of shape `(1, token_size, embedding_size = 768)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct Embeddings(pub(crate) Arc<ArrayBase<OwnedRepr<f32>, Dim<IxDynImpl>>>);

impl Embeddings {
    /// Checks if the embeddings are valid, i.e. finite.
    pub(crate) fn is_valid(&self) -> bool {
        self.iter().copied().all(f32::is_finite)
    }
}

impl Bert {
    /// The range of token sizes.
    pub(crate) const TOKEN_RANGE: RangeInclusive<usize> = 2..=512;

    /// The number of values per embedding.
    pub(crate) const EMBEDDING_SIZE: usize = 768;

    /// Creates a model from an onnx model file.
    ///
    /// Requires the maximum number of tokens per tokenized sequence.
    pub(crate) fn new(model: impl AsRef<Path>, token_size: usize) -> Result<Self, ModelError> {
        if !Self::TOKEN_RANGE.contains(&token_size) {
            return Err(ShapeError::from_kind(ErrorKind::IncompatibleShape).into());
        }

        let environment = Environment::builder().build().unwrap();
        let session = environment
            .new_session_builder()
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::All)
            .unwrap();

        #[cfg(all(target_os = "android"))]
        let session = session.with_nnapi().unwrap();
        // #[cfg(all(target_os = "ios", target_arch = "aarch64"))]
        // let session = session.with_coreml().unwrap();
        let session = session.with_model_from_file(model.as_ref()).unwrap();

        Ok(Bert {
            session,
            token_size,
        })
    }

    /// Runs the model on the encoded sequence to compute the embeddings.
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn run(
        &self,
        token_ids: TokenIds,
        attention_mask: AttentionMask,
    ) -> Result<Embeddings, ModelError> {
        debug_assert_eq!(token_ids.shape(), [1, self.token_size]);
        debug_assert!(token_ids.is_valid(isize::MAX as usize));
        debug_assert_eq!(attention_mask.shape(), [1, self.token_size]);
        debug_assert!(attention_mask.is_valid());
        let inputs = vec![token_ids.0, attention_mask.0];

        let start = Instant::now();
        let outputs = self.session.run(inputs, false).unwrap();
        info!("kpe bert session run time: {:?}", start.elapsed());

        Ok(Embeddings(Arc::new(outputs[0].to_owned())))
    }
}

impl Embeddings {
    /// Collects the valid embeddings according to the mask.
    pub(crate) fn collect(&self, valid_mask: &ValidMask) -> Result<Array2<f32>, ModelError> {
        debug_assert_eq!(self.shape()[0], 1);
        debug_assert_eq!(self.shape()[2], Bert::EMBEDDING_SIZE);
        valid_mask
            .iter()
            .zip(self.rows())
            .filter_map(|(valid, embedding)| valid.then(|| embedding))
            .flatten()
            .copied()
            .collect::<Array1<f32>>()
            .into_shape((valid_mask.count(), self.shape()[2]))
            .map_err(Into::into)
    }
}
