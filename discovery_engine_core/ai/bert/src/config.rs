// Copyright 2022 Xayn AG
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

#[cfg(feature = "japanese")]
use std::path::PathBuf;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    marker::PhantomData,
    path::Path,
};

use displaydoc::Display;
use thiserror::Error;

use crate::{
    model::{BertModel, Model},
    pipeline::{Pipeline, PipelineError},
    tokenizer::Tokenizer,
    NonePooler,
};

/// `BertModel` configuration errors.
#[derive(Debug, Display, Error)]
pub enum ConfigError {
    /// The token size must be greater than two to allow for special tokens
    TokenSize,
    /// Failed to load a data file: {0}
    DataFile(#[from] std::io::Error),
}

/// A `BertModel` configuration.
#[must_use]
pub struct Config<'a, K, P> {
    model_kind: PhantomData<K>,
    vocab: Box<dyn BufRead + Send + 'a>,
    #[cfg(feature = "japanese")]
    japanese: Option<PathBuf>,
    model: Box<dyn Read + Send + 'a>,
    cleanse_accents: bool,
    lower_case: bool,
    token_size: usize,
    pooler: PhantomData<P>,
}

impl<'a, K: BertModel> Config<'a, K, NonePooler> {
    /// Creates a `BertModel` configuration from readables.
    pub fn from_readers(
        vocab: Box<dyn BufRead + Send + 'a>,
        #[cfg(feature = "japanese")] japanese: Option<PathBuf>,
        model: Box<dyn Read + Send + 'a>,
    ) -> Self {
        Config {
            model_kind: PhantomData,
            vocab,
            #[cfg(feature = "japanese")]
            japanese,
            model,
            cleanse_accents: true,
            lower_case: true,
            token_size: 128,
            pooler: PhantomData,
        }
    }

    /// Creates a `BertModel` configuration from files.
    pub fn from_files(
        vocab: impl AsRef<Path>,
        #[cfg(feature = "japanese")] japanese: Option<impl AsRef<Path>>,
        model: impl AsRef<Path>,
    ) -> Result<Self, ConfigError> {
        let vocab = Box::new(BufReader::new(File::open(vocab)?));
        #[cfg(feature = "japanese")]
        let japanese = japanese.map(|japanese| japanese.as_ref().into());
        let model = Box::new(BufReader::new(File::open(model)?));
        Ok(Self::from_readers(
            vocab,
            #[cfg(feature = "japanese")]
            japanese,
            model,
        ))
    }
}

impl<'a, K: BertModel, P> Config<'a, K, P> {
    /// Whether the tokenizer cleanses accents.
    ///
    /// Defaults to `true`.
    pub fn with_cleanse_accents(mut self, cleanse_accents: bool) -> Self {
        self.cleanse_accents = cleanse_accents;
        self
    }

    /// Whether the tokenizer lowercases.
    ///
    /// Defaults to `true`.
    pub fn with_lower_case(mut self, lower_case: bool) -> Self {
        self.lower_case = lower_case;
        self
    }

    /// Sets the token size for the tokenizer and the model.
    ///
    /// Defaults to [`BertModel::TOKEN_RANGE`].
    ///
    /// # Errors
    /// Fails if `size` is less than two or greater than 512.
    pub fn with_token_size(mut self, size: usize) -> Result<Self, ConfigError> {
        if K::TOKEN_RANGE.contains(&size) {
            self.token_size = size;
            Ok(self)
        } else {
            Err(ConfigError::TokenSize)
        }
    }

    /// Sets pooling for the model.
    ///
    /// Defaults to `NonePooler`.
    pub fn with_pooling<NP>(self) -> Config<'a, K, NP> {
        Config {
            vocab: self.vocab,
            #[cfg(feature = "japanese")]
            japanese: self.japanese,
            model: self.model,
            model_kind: self.model_kind,
            cleanse_accents: self.cleanse_accents,
            lower_case: self.lower_case,
            token_size: self.token_size,
            pooler: PhantomData,
        }
    }

    /// Creates a `BertModel` pipeline from a configuration.
    pub fn build(self) -> Result<Pipeline<K, P>, PipelineError> {

        let tokenizer = Tokenizer::new(
            self.vocab,
            #[cfg(feature = "japanese")]
            self.japanese,
            self.cleanse_accents,
            self.lower_case,
            self.token_size,
        )
        .map_err(PipelineError::TokenizerBuild)?;

        let model = Model::new(self.model, self.token_size).map_err(PipelineError::ModelBuild)?;

        Ok(Pipeline {
            tokenizer,
            model,
            pooler: self.pooler,
        })
    }
}
