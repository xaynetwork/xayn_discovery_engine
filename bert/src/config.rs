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

use std::{marker::PhantomData, path::PathBuf};

use figment::{
    error::{Actual, Error, Kind},
    providers::{Format, Toml},
    Figment,
};
use serde::Deserialize;

use crate::{
    model::Model,
    pipeline::{Pipeline, PipelineError},
    pooler::NonePooler,
    tokenizer::{bert::Tokenizer, Tokenize, unigram},
};

/// A pipeline configuration.
///
/// # Example
///
/// The configuration for a Bert pipeline:
///
/// ```toml
/// # the config file is always named `config.toml`
///
/// # optional, eg to enable the japanese pre-tokenizer
/// [pre-tokenizer]
/// path = "mecab"
///
/// # the path is always `vocab.txt`
/// [tokenizer]
/// cleanse-accents = true
/// cleanse-text = true
/// lower-case = false
/// max-chars = 100
///
/// # tokens-related configs of the tokenizer, may differ between tokenizers
/// [tokenizer.tokens]
/// # the `token size` must be in the inclusive range, but is passed as an argument
/// size.min = 2
/// size.max = 512
/// class = "[CLS]"
/// separation = "[SEP]"
/// padding = "[PAD]"
/// unknown = "[UNK]"
/// continuation = "##"
///
/// # the [model] path is always `model.onnx`
///
/// # each input and output is required by tract
/// # string shapes are considered dynamic and depend on arguments
/// [model.input.0]
/// shape.0 = 1
/// shape.1 = "token size"
/// type = "i64"
///
/// [model.input.1]
/// shape.0 = 1
/// shape.1 = "token size"
/// type = "i64"
///
/// [model.input.2]
/// shape.0 = 1
/// shape.1 = "token size"
/// type = "i64"
///
/// [model.output.0]
/// shape.0 = 1
/// shape.1 = "token size"
/// shape.2 = 128
/// type = "f32"
///
/// [model.output.1]
/// shape.0 = 1
/// shape.1 = 128
/// type = "f32"
/// ```
#[must_use]
pub struct Config<T, P> {
    pub dir: PathBuf,
    toml: Figment,
    pub(crate) token_size: usize,
    tokenizer: PhantomData<T>,
    pooler: PhantomData<P>,
}

impl Config<Tokenizer, NonePooler> {
    /// Creates a pipeline configuration.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let dir = dir.into();
        let toml = Figment::from(Toml::file(dir.join("config.toml")));
        let token_size = (toml.extract_inner::<usize>(Self::MIN_TOKEN_SIZE)?
            + toml.extract_inner::<usize>(Self::MAX_TOKEN_SIZE)?)
            / 2;

        Ok(Self {
            dir,
            toml,
            token_size,
            tokenizer: PhantomData,
            pooler: PhantomData,
        })
    }
}

impl Config<unigram::Tokenizer, NonePooler> {
    /// Creates a pipeline configuration.
    pub fn new_unigram(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let dir = dir.into();
        let toml = Figment::from(Toml::file(dir.join("config.toml")));
        let token_size = (toml.extract_inner::<usize>(Self::MIN_TOKEN_SIZE)?
            + toml.extract_inner::<usize>(Self::MAX_TOKEN_SIZE)?)
            / 2;

        Ok(Self {
            dir,
            toml,
            token_size,
            tokenizer: PhantomData,
            pooler: PhantomData,
        })
    }
}


impl<T, P> Config<T, P> {
    const MIN_TOKEN_SIZE: &str = "tokenizer.tokens.size.min";
    const MAX_TOKEN_SIZE: &str = "tokenizer.tokens.size.max";

    pub fn extract<'b, V>(&self, key: &str) -> Result<V, Error>
    where
        V: Deserialize<'b>,
    {
        self.toml.extract_inner(key).map_err(Into::into)
    }

    /// Sets the token size for the tokenizer and the model.
    ///
    /// Defaults to the midpoint of the token size range.
    ///
    /// # Errors
    /// Fails if `size` is not within the token size range.
    pub fn with_token_size(mut self, size: usize) -> Result<Self, Error> {
        let min = self.extract::<usize>(Self::MIN_TOKEN_SIZE)?;
        let max = self.extract::<usize>(Self::MAX_TOKEN_SIZE)?;

        if (min..=max).contains(&size) {
            self.token_size = size;
            Ok(self)
        } else {
            Err(Error::from(Kind::InvalidValue(
                Actual::Unsigned(size as u128),
                format!("{min}..={max}"),
            )))
        }
    }

    /// Sets the tokenizer for the model.
    ///
    /// Defaults to `bert::Tokenizer`.
    pub fn with_tokenizer<U>(self) -> Config<U, P> {
        Config {
            dir: self.dir,
            toml: self.toml,
            token_size: self.token_size,
            tokenizer: PhantomData,
            pooler: self.pooler,
        }
    }

    /// Sets the pooler for the model.
    ///
    /// Defaults to `NonePooler`.
    pub fn with_pooler<Q>(self) -> Config<T, Q> {
        Config {
            dir: self.dir,
            toml: self.toml,
            token_size: self.token_size,
            tokenizer: self.tokenizer,
            pooler: PhantomData,
        }
    }

    /// Creates a pipeline from a configuration.
    pub fn build(&self) -> Result<Pipeline<T, P>, PipelineError>
    where
        T: Tokenize,
    {
        let tokenizer = T::new(self)?;
        let model = Model::new(self)?;

        Ok(Pipeline {
            tokenizer,
            model,
            pooler: self.pooler,
        })
    }
}
