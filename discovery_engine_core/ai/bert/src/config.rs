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

use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    marker::PhantomData,
    path::Path,
};

use displaydoc::Display;
use thiserror::Error;

use crate::{model::BertModel, NonePooler};
use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

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
    pub(crate) model_kind: PhantomData<K>,
    pub(crate) vocab: Box<dyn BufRead + Send + 'a>,
    pub(crate) model: Box<dyn Read + Send + 'a>,
    pub(crate) accents: AccentChars,
    pub(crate) case: CaseChars,
    pub(crate) token_size: usize,
    pub(crate) pooler: PhantomData<P>,
}

impl<'a, K: BertModel> Config<'a, K, NonePooler> {
    /// Creates a `BertModel` configuration from readables.
    pub fn from_readers(
        vocab: Box<dyn BufRead + Send + 'a>,
        model: Box<dyn Read + Send + 'a>,
    ) -> Self {
        Config {
            model_kind: PhantomData,
            vocab,
            model,
            accents: AccentChars::Cleanse,
            case: CaseChars::Lower,
            token_size: 128,
            pooler: PhantomData,
        }
    }

    /// Creates a `BertModel` configuration from files.
    pub fn from_files(
        vocab: impl AsRef<Path>,
        model: impl AsRef<Path>,
    ) -> Result<Self, ConfigError> {
        let vocab = Box::new(BufReader::new(File::open(vocab)?));
        let model = Box::new(BufReader::new(File::open(model)?));
        Ok(Self::from_readers(vocab, model))
    }
}

impl<'a, K: BertModel, P> Config<'a, K, P> {
    /// Whether the tokenizer keeps accents.
    ///
    /// Defaults to `AccentChars::Cleanse`.
    pub fn with_accents(mut self, accents: AccentChars) -> Self {
        self.accents = accents;
        self
    }

    /// Whether the tokenizer lowercases.
    ///
    /// Defaults to `CaseChars::Lower`.
    pub fn with_case(mut self, case: CaseChars) -> Self {
        self.case = case;
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
            model: self.model,
            model_kind: self.model_kind,
            accents: self.accents,
            case: self.case,
            token_size: self.token_size,
            pooler: PhantomData,
        }
    }
}
