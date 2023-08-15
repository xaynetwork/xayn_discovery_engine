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
use serde::{Deserialize, Serialize};

use crate::{
    model::Model,
    pipeline::{Pipeline, PipelineError},
    pooler::NonePooler,
    tokenizer::Tokenizer,
};

#[derive(Clone, Debug)]
pub enum Runtime {
    Tract,
    Ort(PathBuf),
}

impl Runtime {
    const TRACT: &str = "tract";
}

impl<'de> Deserialize<'de> for Runtime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match String::deserialize(deserializer)? {
            tract if tract == Self::TRACT => Ok(Self::Tract),
            ort => Ok(Self::Ort(ort.into())),
        }
    }
}

impl Serialize for Runtime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Tract => Self::TRACT.serialize(serializer),
            Self::Ort(ort) => ort.serialize(serializer),
        }
    }
}

/// A pipeline configuration.
///
/// # Example
///
/// The configuration for a Bert pipeline:
///
/// ```toml
/// # the config file is always named `config.toml`
///
/// # the path is always `tokenizer.json`
/// [tokenizer]
/// add-special-tokens = true
///
/// [tokenizer.tokens]
/// # the `token size` must be in the inclusive range, but is passed as an argument
/// size.min = 2
/// size.max = 512
/// padding = "[PAD]"
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
pub struct Config<P> {
    pub(crate) dir: PathBuf,
    toml: Figment,
    pub(crate) token_size: usize,
    pub(crate) runtime: Runtime,
    pooler: PhantomData<P>,
}

impl Config<NonePooler> {
    /// Creates a pipeline configuration.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, Error> {
        let dir = dir.into();
        if !dir.exists() {
            return Err(Error::from(Kind::Message(format!(
                "embedder directory '{}' doesn't exist",
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

        let toml = Figment::from(Toml::file(toml));
        let token_size = (toml.extract_inner::<usize>(Self::MIN_TOKEN_SIZE)?
            + toml.extract_inner::<usize>(Self::MAX_TOKEN_SIZE)?)
            / 2;

        Ok(Self {
            dir,
            toml,
            token_size,
            runtime: Runtime::Tract,
            pooler: PhantomData,
        })
    }
}

impl<P> Config<P> {
    const MIN_TOKEN_SIZE: &str = "tokenizer.tokens.size.min";
    const MAX_TOKEN_SIZE: &str = "tokenizer.tokens.size.max";

    pub fn extract<'b, V>(&self, key: &str) -> Result<V, Error>
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

    /// Sets the token size for the tokenizer and the model.
    ///
    /// Defaults to the midpoint of the token size range.
    ///
    /// # Errors
    /// Fails if `size` is not within the token size range.
    pub fn with_token_size(mut self, size: usize) -> Result<Self, Error> {
        self.token_size = size;
        self.validate()?;

        Ok(self)
    }

    /// Sets the runtime for the model.
    ///
    /// Defaults to `Tract`.
    pub fn with_runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;

        self
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
        let dir = match &self.runtime {
            Runtime::Tract => {
                return Err(Error::from(Kind::Message(
                    "embedder runtime isn't available for tract".into(),
                )))
            }
            Runtime::Ort(dir) => dir,
        };
        cfg_if! {
            if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
                let runtime = dir.join("linux_aarch64/lib/libonnxruntime.so");
            } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
                let runtime = dir.join("linux_x64/lib/libonnxruntime.so");
            } else if #[cfg(target_os = "macos")] {
                let runtime = dir.join("macos/lib/libonnxruntime.dylib");
            } else {
                return Err(Error::from(Kind::Message(
                    "embedder runtime isn't available for this target os/arch".into(),
                )));
            }
        }

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
