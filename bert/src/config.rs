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

use cfg_if::cfg_if;
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
    tokenizer::Tokenizer,
};

/// A pipeline configuration.
///
/// # Example
///
/// The configuration for a Bert pipeline:
///
/// ```toml
/// # the config file is always named `config.toml`
/// # the tokenizer file is always named `tokenizer.json`
/// # the model file is always named `model.onnx`
/// # the runtime file is always named `lib/libonnxruntime.<so/dylib>`
///
/// [tokenizer]
/// add-special-tokens = true
/// # the `token size` must be in the inclusive range, but is passed as an argument
/// size.min = 2
/// size.max = 512
/// padding = "[PAD]"
/// ```
#[must_use]
pub struct Config<P> {
    pub(crate) dir: PathBuf,
    toml: Figment,
    pub(crate) token_size: usize,
    pub(crate) runtime: PathBuf,
    pooler: PhantomData<P>,
}

impl Config<NonePooler> {
    /// Creates a pipeline configuration.
    pub fn new(dir: impl Into<PathBuf>, runtime: impl Into<PathBuf>) -> Result<Self, Error> {
        let dir = dir.into();
        if !dir.exists() {
            return Err(Error::from(Kind::Message(format!(
                "embedder model directory '{}' doesn't exist",
                dir.display(),
            ))));
        }
        let toml = dir.join("config.toml");
        if !toml.exists() {
            return Err(Error::from(Kind::Message(format!(
                "embedder config '{}' doesn't exist",
                toml.display(),
            ))));
        }
        let runtime = runtime.into();
        if !runtime.exists() {
            return Err(Error::from(Kind::Message(format!(
                "embedder runtime directory '{}' doesn't exist",
                dir.display(),
            ))));
        }

        let toml = Figment::from(Toml::file(toml));
        let token_size = (toml.extract_inner::<usize>(Self::MIN_TOKEN_SIZE)?
            + toml.extract_inner::<usize>(Self::MAX_TOKEN_SIZE)?)
            / 2;

        Ok(Self {
            dir,
            toml,
            token_size,
            runtime,
            pooler: PhantomData,
        })
    }
}

impl<P> Config<P> {
    const MIN_TOKEN_SIZE: &'static str = "tokenizer.min_size";
    const MAX_TOKEN_SIZE: &'static str = "tokenizer.max_size";

    pub(crate) fn extract<'b, V>(&self, key: &str) -> Result<V, Error>
    where
        V: Deserialize<'b>,
    {
        self.toml.extract_inner(key).map_err(Into::into)
    }

    pub fn validate(&self) -> Result<(), Error> {
        let min = self.extract::<usize>(Self::MIN_TOKEN_SIZE)?;
        let max = self.extract::<usize>(Self::MAX_TOKEN_SIZE)?;
        if !(min..=max).contains(&self.token_size) {
            return Err(Error::from(Kind::InvalidValue(
                Actual::Unsigned(self.token_size as u128),
                format!("token_size in {min}..={max}"),
            )));
        }

        Ok(())
    }

    /// Sets the token size for the tokenizer.
    ///
    /// Too long tokenized sequences are truncated accordingly. Defaults to the midpoint of the
    /// token size range.
    ///
    /// # Errors
    /// Fails if `size` is not within the token size range.
    pub fn with_token_size(mut self, size: usize) -> Result<Self, Error> {
        self.token_size = size;
        self.validate()?;

        Ok(self)
    }

    /// Sets the pooler for the model.
    ///
    /// Defaults to `NonePooler`.
    pub fn with_pooler<Q>(self) -> Config<Q> {
        Config {
            dir: self.dir,
            toml: self.toml,
            token_size: self.token_size,
            runtime: self.runtime,
            pooler: PhantomData,
        }
    }

    pub(crate) fn model(&self) -> Result<PathBuf, Error> {
        let model = self.dir.join("model.onnx");

        if model.exists() {
            Ok(model)
        } else {
            Err(Error::from(Kind::Message(format!(
                "embedder model '{}' doesn't exist",
                model.display(),
            ))))
        }
    }

    pub(crate) fn runtime(&self) -> Result<PathBuf, Error> {
        cfg_if! {
            if #[cfg(target_os = "linux")] {
                let extension = "so";
            } else if #[cfg(target_os = "macos")] {
                let extension = "dylib";
            } else {
                return Err(Error::from(Kind::Message(
                    "embedder runtime isn't available for this target os".into(),
                )));
            }
        }
        let runtime = self.runtime.join(format!("lib/libonnxruntime.{extension}"));

        if runtime.exists() {
            Ok(runtime)
        } else {
            Err(Error::from(Kind::Message(format!(
                "embedder runtime '{}' doesn't exist",
                runtime.display(),
            ))))
        }
    }

    /// Creates a pipeline from a configuration.
    pub fn build(&self) -> Result<Pipeline<P>, PipelineError> {
        let tokenizer = Tokenizer::new(self)?;
        let model = Model::new(self)?;

        Ok(Pipeline {
            tokenizer,
            model,
            pooler: self.pooler,
        })
    }
}
