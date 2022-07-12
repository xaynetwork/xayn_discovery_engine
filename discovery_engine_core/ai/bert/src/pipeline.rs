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
    model::{BertModel, Model, ModelError},
    pooler::{Embedding1, Embedding2, PoolerError},
    tokenizer::{Tokenizer, TokenizerError},
    AveragePooler,
    FirstPooler,
    NonePooler,
};

/// A pipeline for a bert model.
///
/// Can be built from a [`Config`] and consists of a tokenizer, a model and a pooler.
///
/// [`Config`]: crate::config::Config
pub struct Pipeline<K, P> {
    pub(crate) tokenizer: Tokenizer,
    pub(crate) model: Model<K>,
    pub(crate) pooler: PhantomData<P>,
}

/// The potential errors of the [`Pipeline`].
#[derive(Debug, Display, Error)]
pub enum PipelineError {
    /// Failed to run the tokenizer: {0}
    Tokenizer(#[from] TokenizerError),
    /// Failed to run the model: {0}
    Model(#[from] ModelError),
    /// Failed to run the pooler: {0}
    Pooler(#[from] PoolerError),
    /// Failed to build the tokenizer: {0}
    TokenizerBuild(#[source] TokenizerError),
    /// Failed to build the model: {0}
    ModelBuild(#[source] ModelError),
}

impl<K> Pipeline<K, NonePooler>
where
    K: BertModel,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding2, PipelineError> {
        let encoding = self.tokenizer.encode(sequence);
        let prediction = self.model.predict(encoding)?;
        NonePooler::pool(&prediction).map_err(Into::into)
    }
}

impl<K> Pipeline<K, FirstPooler>
where
    K: BertModel,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence);
        let prediction = self.model.predict(encoding)?;
        FirstPooler::pool(&prediction).map_err(Into::into)
    }
}

impl<K> Pipeline<K, AveragePooler>
where
    K: BertModel,
{
    /// Computes the embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence);
        let attention_mask = encoding.attention_mask.clone();
        let prediction = self.model.predict(encoding)?;
        AveragePooler::pool(&prediction, &attention_mask).map_err(Into::into)
    }
}

impl<K, P> Pipeline<K, P>
where
    K: BertModel,
{
    /// Gets the token size.
    pub fn token_size(&self) -> usize {
        self.model.token_size
    }

    /// Gets the embedding size.
    pub fn embedding_size() -> usize {
        K::EMBEDDING_SIZE
    }
}

#[cfg(test)]
mod tests {
    use xayn_discovery_engine_test_utils::smbert::{model, vocab};

    use super::*;
    use crate::{
        config::Config,
        model::kinds::SMBert,
        pooler::{AveragePooler, FirstPooler, NonePooler},
    };

    fn pipeline<P>() -> Pipeline<SMBert, P> {
        Config::from_files(vocab().unwrap(), model().unwrap())
            .unwrap()
            .with_pooling()
            .build()
            .unwrap()
    }

    #[test]
    fn test_pipeline_none() {
        let pipeline = pipeline::<NonePooler>();
        let shape = [
            pipeline.token_size(),
            Pipeline::<SMBert, NonePooler>::embedding_size(),
        ];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }

    #[test]
    fn test_pipeline_first() {
        let pipeline = pipeline::<FirstPooler>();
        let shape = [Pipeline::<SMBert, FirstPooler>::embedding_size()];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }

    #[test]
    fn test_pipeline_average() {
        let pipeline = pipeline::<AveragePooler>();
        let shape = [crate::SMBert::embedding_size()];

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), shape);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), shape);
    }
}
