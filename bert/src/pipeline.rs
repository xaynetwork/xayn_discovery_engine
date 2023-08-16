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
    model::Model,
    pooler::{Embedding1, Embedding2},
    tokenizer::Tokenizer,
    AveragePooler,
    FirstPooler,
    NonePooler,
};

/// A pipeline can be built from a [`Config`] and consists of a tokenizer, a model and a pooler.
///
/// [`Config`]: crate::config::Config
pub struct Pipeline<P> {
    pub(crate) tokenizer: Tokenizer,
    pub(crate) model: Model,
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
    Model(#[from] anyhow::Error),
}

impl Pipeline<NonePooler> {
    /// Computes the pooled embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding2, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let embedding = self.model.embed(&encoding)?;
        let pooling = NonePooler::pool(&embedding.extract()?.view());

        Ok(pooling)
    }
}

impl Pipeline<FirstPooler> {
    /// Computes the pooled embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let embedding = self.model.embed(&encoding)?;
        let pooling = FirstPooler::pool(&embedding.extract()?.view());

        Ok(pooling)
    }
}

impl Pipeline<AveragePooler> {
    /// Computes the pooled embedding of the sequence.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<Embedding1, PipelineError> {
        let encoding = self.tokenizer.encode(sequence)?;
        let embedding = self.model.embed(&encoding)?;
        let pooling = AveragePooler::pool(&embedding.extract()?.view(), &encoding);

        Ok(pooling)
    }
}

impl<P> Pipeline<P> {
    /// Gets the embedding size.
    pub fn embedding_size(&self) -> usize {
        self.model.embedding_size
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use xayn_test_utils::asset::{e5_mocked, ort, smbert_mocked};

    use super::*;
    use crate::{
        config::Config,
        pooler::{AveragePooler, FirstPooler, NonePooler},
    };

    fn pipeline<P>(dir: PathBuf) -> Pipeline<P> {
        Config::new(dir, ort().unwrap())
            .unwrap()
            .with_pooler()
            .build()
            .unwrap()
    }

    #[test]
    fn test_pipeline_none() {
        let pipeline = pipeline::<NonePooler>(smbert_mocked().unwrap());

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), [7, pipeline.embedding_size()]);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), [2, pipeline.embedding_size()]);
    }

    #[test]
    fn test_pipeline_first() {
        let pipeline = pipeline::<FirstPooler>(smbert_mocked().unwrap());

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);
    }

    #[test]
    fn test_pipeline_average() {
        let pipeline = pipeline::<AveragePooler>(smbert_mocked().unwrap());

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);
    }

    #[test]
    fn test_e5_pipeline() {
        let pipeline = pipeline::<AveragePooler>(e5_mocked().unwrap());

        let embeddings = pipeline.run("This is a sequence.").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);

        let embeddings = pipeline.run("").unwrap();
        assert_eq!(embeddings.shape(), [pipeline.embedding_size()]);
    }
}
