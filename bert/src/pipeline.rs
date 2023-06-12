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

use std::marker::PhantomData;

use displaydoc::Display;
use thiserror::Error;

use crate::{
    embedding::{Embedding1, Embedding2, SparseEmbedding},
    model::{Model, SparseModel},
    pooler::{AveragePooler, FirstPooler, NonePooler},
    tokenizer::Tokenize,
    InvalidEmbedding,
};

/// A pipeline can be built from a [`Config`] and consists of a tokenizer, a model and a pooler.
///
/// [`Config`]: crate::config::Config
pub struct Pipeline<T, P> {
    pub(crate) tokenizer: T,
    pub(crate) model: Model,
    pub(crate) sparse_model: SparseModel,
    pub(crate) pooler: PhantomData<P>,
}

/// The potential errors of the [`Pipeline`].
#[derive(Debug, Display, Error)]
#[allow(clippy::large_enum_variant)]
pub enum PipelineError {
    /// Failed to configure the pipeline: {0}
    Config(#[from] figment::Error),
    /// Failed to run the tokenizer: {0}
    Tokenizer(#[from] tokenizers::Error),
    /// Failed to run the model: {0}
    Model(#[from] tract_onnx::prelude::TractError),
    /// {0}
    InvalidEmbedding(#[from] InvalidEmbedding),
}

impl<T> Pipeline<T, NonePooler>
where
    T: Tokenize,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding2, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let prediction = self.model.predict(encoding.to_model_input())?;

        NonePooler::pool(&prediction).map_err(Into::into)
    }
}

impl<T> Pipeline<T, FirstPooler>
where
    T: Tokenize,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let prediction = self.model.predict(encoding.to_model_input())?;

        FirstPooler::pool(&prediction).map_err(Into::into)
    }
}

impl<T> Pipeline<T, AveragePooler>
where
    T: Tokenize,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let prediction = self.model.predict(encoding.to_model_input())?;

        AveragePooler::pool(&prediction, encoding.get_attention_mask()).map_err(Into::into)
    }
}

impl<T, P> Pipeline<T, P>
where
    T: Tokenize,
{
    /// Computes the sparse embedding of the document sequence.
    pub fn run_sparse_document(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<SparseEmbedding, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let frequency = encoding
            .to_token_frequency(self.tokenizer.special_token_ids())
            .map_err(|_| InvalidEmbedding)?;
        let (indices, values) = self.sparse_model.run_document(frequency);

        SparseEmbedding::new(indices, values).map_err(Into::into)
    }

    /// Computes the sparse embedding of the query sequence.
    pub fn run_sparse_query(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<SparseEmbedding, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let frequency = encoding
            .to_token_ids(self.tokenizer.special_token_ids())
            .map_err(|_| InvalidEmbedding)?;
        let (indices, values) = self.sparse_model.run_query(frequency);

        SparseEmbedding::new(indices, values).map_err(Into::into)
    }
}

impl<T, P> Pipeline<T, P> {
    /// Gets the token size.
    pub fn token_size(&self) -> usize {
        self.model.token_size
    }

    /// Gets the embedding size.
    pub fn embedding_size(&self) -> usize {
        self.model.embedding_size
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::smbert_mocked;

    use super::*;
    use crate::{
        config::Config,
        pooler::{AveragePooler, FirstPooler, NonePooler},
        tokenizer::bert::Tokenizer,
    };

    fn pipeline<P>() -> Pipeline<Tokenizer, P> {
        Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_pooler()
            .build()
            .unwrap()
    }

    #[test]
    #[ignore = "sparse model not available"]
    fn test_pipeline_none() {
        let pipeline = pipeline::<NonePooler>();
        let shape = [pipeline.token_size(), pipeline.embedding_size()];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }

    #[test]
    #[ignore = "sparse model not available"]
    fn test_pipeline_first() {
        let pipeline = pipeline::<FirstPooler>();
        let shape = [pipeline.embedding_size()];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }

    #[test]
    #[ignore = "sparse model not available"]
    fn test_pipeline_average() {
        let pipeline = pipeline::<AveragePooler>();
        let shape = [pipeline.embedding_size()];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }
}
