// Copyright 2023 Xayn AG
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

#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]
#![deny(
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications,
    unsafe_op_in_unsafe_fn
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

pub mod pool;
mod python_child;

use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use displaydoc::Display;
use python_child::{PipeCommand, PythonChild};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Display, Error)]
pub enum Error {
    /// Serializing message for snippet extractor failed: {0}
    Serialization(#[from] rmp_serde::encode::Error),
    /// Deserializing message from snippet extractor failed: {0}
    Deserialization(#[from] rmp_serde::decode::Error),
    /// Communication with snippet extractor failed: {0}
    Io(#[from] io::Error),
    /// Snippet extraction failed: {msg}
    SnippetExtractionFailed { msg: String },
    /// Loading tokenizer failed: {msg}
    LoadingTokenizerFailed { msg: String },
    /// Unknown Tokenizer: {name}
    UnknownTokenizer { name: String },
}

impl Error {
    fn can_child_be_reused(&self) -> bool {
        matches!(
            self,
            Error::SnippetExtractionFailed { .. }
                | Error::LoadingTokenizerFailed { .. }
                | Error::UnknownTokenizer { .. }
                | Error::Serialization { .. }
        )
    }
}

/// Configurations of the coi system.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[must_use]
pub struct Config {
    pub use_pipenv: bool,
    pub python_workspace: PathBuf,
    pub language: String,
    pub chunk_size: usize,
    pub hard_chunk_size_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            use_pipenv: false,
            language: "english".into(),
            chunk_size: 500,
            hard_chunk_size_limit: 520,
            python_workspace: "./".into(),
        }
    }
}

type StringPath = String;

pub struct SnippetExtractor {
    config: Config,
    // Hint: If we ever allow unloading tokenizers this needs to be a map.
    tokenizers: HashMap<String, StringPath>,
    child: Option<PythonChild>,
}

impl SnippetExtractor {
    const DEFAULT_TOKENIZER_NAME: &str = "t0";

    pub fn new_with_tokenizer(
        config: Config,
        tokenizer_file: impl AsRef<Path>,
    ) -> Result<Self, Error> {
        let mut this = Self::new(config);
        this.add_tokenizer(Self::DEFAULT_TOKENIZER_NAME, tokenizer_file.as_ref())?;
        Ok(this)
    }

    pub fn new(config: Config) -> Self {
        Self {
            config,
            tokenizers: HashMap::new(),
            child: None,
        }
    }

    pub fn add_tokenizer(&mut self, name: &str, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| Error::LoadingTokenizerFailed {
                msg: "tokenizer path needs to be utf-8".into(),
            })?;

        if self.child.is_some() {
            self.with_child(|child, _config| {
                child.send_command(&LoadTokenizer { name, path }, |msg| {
                    Error::LoadingTokenizerFailed { msg }
                })
            })?;
        }

        self.tokenizers.insert(name.to_owned(), path.to_owned());

        Ok(())
    }

    pub fn extract_snippet(&mut self, document: &str) -> Result<Vec<String>, Error> {
        self.extract_snippet_with_tokenizer(Self::DEFAULT_TOKENIZER_NAME, document)
    }

    pub fn extract_snippet_with_tokenizer(
        &mut self,
        tokenizer: &str,
        document: &str,
    ) -> Result<Vec<String>, Error> {
        if !self.tokenizers.contains_key(tokenizer) {
            return Err(Error::UnknownTokenizer {
                name: tokenizer.into(),
            });
        }

        self.with_child(|child, config| {
            child.send_command(
                &Extract {
                    language: &config.language,
                    chunk_size: config.chunk_size,
                    hard_chunk_size_limit: config.hard_chunk_size_limit,
                    tokenizer,
                    document,
                },
                |msg| Error::SnippetExtractionFailed { msg },
            )
        })
    }

    fn with_child<V>(
        &mut self,
        func: impl FnOnce(&mut PythonChild, &Config) -> Result<V, Error>,
    ) -> Result<V, Error> {
        let mut child = self.take_child()?;
        let res = func(&mut child, &self.config);
        match res {
            Err(err) if !err.can_child_be_reused() => Err(err),
            reusable => {
                self.child = Some(child);
                reusable
            }
        }
    }

    fn take_child(&mut self) -> Result<PythonChild, Error> {
        if let Some(mut child) = self.child.take() {
            if child.send_command(&Ping {}, |_| DiscardError).is_ok() {
                Ok(child)
            } else {
                self.spawn_child()
            }
        } else {
            self.spawn_child()
        }
    }

    fn spawn_child(&self) -> Result<PythonChild, Error> {
        let mut child = PythonChild::spawn(
            &self.config.python_workspace,
            "./python_src/snippet_extractor.py",
            self.config.use_pipenv,
        )?;

        let ready = child.read_message::<String, Error>()?;
        assert_eq!(ready, "ready");

        for (name, path) in &self.tokenizers {
            child.send_command(&LoadTokenizer { name, path }, |msg| {
                Error::LoadingTokenizerFailed { msg }
            })?;
        }

        Ok(child)
    }
}

impl Drop for SnippetExtractor {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            let mut child = child.into_child_dropping_pipes();
            let start = Instant::now();
            // we dropped stdin on which the child reads so exit is imminent
            while child.try_wait().is_ok_and(|exited| exited.is_none()) {
                if Instant::now().duration_since(start) > Duration::from_millis(150) {
                    child.kill().ok();
                }
                thread::yield_now();
            }
        }
    }
}

#[derive(Serialize)]
struct Ping {}

impl PipeCommand for Ping {
    type Value = bool;
    const TAG: &'static str = "ping";
}

#[derive(Serialize)]
struct LoadTokenizer<'a> {
    name: &'a str,
    path: &'a str,
}

impl PipeCommand for LoadTokenizer<'_> {
    type Value = bool;
    const TAG: &'static str = "initialize_tokenizer";
}

#[derive(Serialize)]
struct Extract<'a> {
    language: &'a str,
    chunk_size: usize,
    hard_chunk_size_limit: usize,
    tokenizer: &'a str,
    document: &'a str,
}

impl PipeCommand for Extract<'_> {
    type Value = Vec<String>;
    const TAG: &'static str = "extract";
}

struct DiscardError;

impl From<rmp_serde::encode::Error> for DiscardError {
    fn from(_: rmp_serde::encode::Error) -> Self {
        Self
    }
}

impl From<rmp_serde::decode::Error> for DiscardError {
    fn from(_: rmp_serde::decode::Error) -> Self {
        Self
    }
}

impl From<io::Error> for DiscardError {
    fn from(_: io::Error) -> Self {
        Self
    }
}
