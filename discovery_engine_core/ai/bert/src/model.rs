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
    io::Error as IoError,
    marker::PhantomData,
    ops::RangeInclusive,
    path::Path,
    sync::Arc,
    time::Instant,
};

use derive_more::{Deref, From};
use displaydoc::Display;
use ndarray::{ArrayBase, Dim, ErrorKind, IxDynImpl, OwnedRepr, ShapeError};
use onnxruntime::{environment::Environment, session::Session, GraphOptimizationLevel};
use thiserror::Error;
use tracing::info;
use tract_onnx::prelude::TractError;

use crate::tokenizer::Encoding;

pub mod kinds {
    //! Types [`SMBert`] and [`QAMBert`] represent the kind of model that we want.
    //! It must be passed together with `vocab` and `model` parameters.
    //! Passing the wrong kind with respect to the model can lead to a wrong output of the pipeline.

    use std::ops::RangeInclusive;

    use super::BertModel;

    /// Sentence (Embedding) Multilingual Bert
    #[derive(Debug)]
    pub struct SMBert;

    impl BertModel for SMBert {
        const TOKEN_RANGE: RangeInclusive<usize> = 2..=512;
        const EMBEDDING_SIZE: usize = 128;
    }

    /// Question Answering (Embedding) Multilingual Bert
    #[derive(Debug)]
    pub struct QAMBert;

    impl BertModel for QAMBert {
        const TOKEN_RANGE: RangeInclusive<usize> = 2..=512;
        const EMBEDDING_SIZE: usize = 128;
    }
}

/// A Bert onnx model.
#[derive(Debug)]
pub(crate) struct Model<K> {
    session: Session,
    pub(crate) token_size: usize,
    _kind: PhantomData<K>,
}

/// The potential errors of the model.
#[derive(Debug, Display, Error)]
pub enum ModelError {
    /// Failed to read the onnx model: {0}
    Read(#[from] IoError),
    /// Failed to run a tract operation: {0}
    Tract(#[from] TractError),
    /// Invalid onnx model shapes: {0}
    Shape(#[from] ShapeError),
}

/// Properties for kinds of Bert models.
pub trait BertModel: Sized {
    /// The range of token sizes.
    const TOKEN_RANGE: RangeInclusive<usize>;

    /// The number of values per embedding.
    const EMBEDDING_SIZE: usize;
}

/// The predicted encoding.
///
/// The prediction is of shape `(1, token_size, embedding_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct Prediction(Arc<ArrayBase<OwnedRepr<f32>, Dim<IxDynImpl>>>);

impl<K> Model<K>
where
    K: BertModel,
{
    /// Creates a model from an onnx model file.
    ///
    /// Requires the maximum number of tokens per tokenized sequence.
    pub(crate) fn new(
        // `Read` instead of `AsRef<Path>` is needed for wasm
        model: impl AsRef<Path>,
        token_size: usize,
    ) -> Result<Self, ModelError> {
        if !K::TOKEN_RANGE.contains(&token_size) {
            return Err(ShapeError::from_kind(ErrorKind::IncompatibleShape).into());
        }

        let environment = Environment::builder().build().unwrap();
        let session = environment
            .new_session_builder()
            .unwrap()
            .with_number_intra_threads(1)
            .unwrap()
            .with_number_inter_threads(1)
            .unwrap()
            .with_optimization_level(GraphOptimizationLevel::All)
            .unwrap();

        // #[cfg(all(target_os = "android", target_arch = "aarch64"))]
        // let session = session.with_nnapi().unwrap();
        let session = session.with_model_from_file(model.as_ref()).unwrap();

        Ok(Model {
            session,
            token_size,
            _kind: PhantomData,
        })
    }

    /// Runs prediction on the encoded sequence.
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn predict(&self, encoding: Encoding) -> Result<Prediction, ModelError> {
        debug_assert_eq!(encoding.token_ids.shape(), [1, self.token_size]);
        debug_assert_eq!(encoding.attention_mask.shape(), [1, self.token_size]);
        debug_assert_eq!(encoding.type_ids.shape(), [1, self.token_size]);
        let inputs = vec![
            encoding.token_ids.0,
            encoding.attention_mask.0,
            encoding.type_ids.0,
        ];

        let start = Instant::now();
        let outputs = self.session.run(inputs, false).unwrap();
        info!("smbert session run time: {:?}", start.elapsed());

        Ok(Prediction(Arc::new(outputs[0].to_owned())))
    }
}

#[cfg(test)]
mod tests {
    use ndarray::Array2;
    use std::{fs::File, io::BufReader};

    use xayn_discovery_engine_test_utils::smbert::model;

    use super::*;

    #[test]
    fn test_model_shapes() {
        assert_eq!(kinds::SMBert::TOKEN_RANGE, 2..=512);
        assert_eq!(kinds::SMBert::EMBEDDING_SIZE, 128);

        assert_eq!(kinds::QAMBert::TOKEN_RANGE, 2..=512);
        assert_eq!(kinds::QAMBert::EMBEDDING_SIZE, 128);
    }

    // #[test]
    // fn test_model_empty() {
    //     assert!(matches!(
    //         Model::<kinds::SMBert>::new(Vec::new().as_slice(), 10).unwrap_err(),
    //         ModelError::Tract(_),
    //     ));
    // }

    // #[test]
    // fn test_model_invalid() {
    //     assert!(matches!(
    //         Model::<kinds::SMBert>::new([0].as_ref(), 10).unwrap_err(),
    //         ModelError::Tract(_),
    //     ));
    // }

    // #[test]
    // fn test_token_size_invalid() {
    //     let model = BufReader::new(File::open(model().unwrap()).unwrap());
    //     assert!(matches!(
    //         Model::<kinds::SMBert>::new(model, 0).unwrap_err(),
    //         ModelError::Shape(_),
    //     ));
    // }

    // #[test]
    // fn test_predict() {
    //     let shape = (1, 64);
    //     let model = BufReader::new(File::open(model().unwrap()).unwrap());
    //     let model = Model::<kinds::SMBert>::new(model, shape.1).unwrap();

    //     let encoding = Encoding {
    //         token_ids: Array2::from_elem(shape, 0).into(),
    //         attention_mask: Array2::from_elem(shape, 1).into(),
    //         type_ids: Array2::from_elem(shape, 0).into(),
    //     };
    //     let prediction = model.predict(encoding).unwrap();
    //     assert_eq!(
    //         prediction.shape(),
    //         [shape.0, shape.1, kinds::SMBert::EMBEDDING_SIZE],
    //     );
    // }
}
