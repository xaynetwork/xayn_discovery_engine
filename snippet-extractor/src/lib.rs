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

//! This library provides functionality to extract snippets from a document.

pub mod pool;
mod python_child;

use std::{collections::HashMap, io, path::PathBuf};

use displaydoc::Display;
use python_child::{PipeCommand, PythonChild};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;

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
    /// Unexpected error response: {msg}
    UnexpectedErrorResponse { msg: String },
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[must_use]
pub struct Config {
    pub python_workspace: PathBuf,
    pub language: String,
    // TODO[pmk/now] use relative path buf
    pub tokenizers: HashMap<String, PathBuf>,
    pub chunk_size: usize,
    pub hard_chunk_size_limit: usize,
    // Hint: From a per-crate design POV this shouldn't be a member of Config,
    //       but from a application level POV this is much more convenient.
    pub pool: pool::Config,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "english".into(),
            chunk_size: 500,
            hard_chunk_size_limit: 520,
            tokenizers: [("default".into(), "./assets/tokenizer.json".into())].into(),
            python_workspace: "./".into(),
            pool: pool::Config::default(),
        }
    }
}

pub struct SnippetExtractor {
    config: Config,
    child: Option<PythonChild>,
}

impl SnippetExtractor {
    pub fn new(config: Config) -> Result<Self, Error> {
        for (name, path) in &config.tokenizers {
            if path.to_str().is_none() {
                return Err(Error::LoadingTokenizerFailed {
                    msg: format!("tokenizer ({name}) path needs to be utf-8"),
                });
            }
        }

        Ok(Self {
            config,
            child: None,
        })
    }

    pub fn force_initialization(&mut self) -> Result<(), Error> {
        self.with_child(|_child, _config| Ok(()))
    }

    pub fn extract_snippet(
        &mut self,
        tokenizer: &str,
        document: &str,
    ) -> Result<Vec<String>, Error> {
        if !self.config.tokenizers.contains_key(tokenizer) {
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
            Err(err) if !err.can_child_be_reused() => {
                error!("discarding snippet extractor child process");
                Err(err)
            }
            reusable => {
                self.child = Some(child);
                reusable
            }
        }
    }

    fn take_child(&mut self) -> Result<PythonChild, Error> {
        if let Some(mut child) = self.child.take() {
            match child.send_command(&Ping {}, |msg| Error::UnexpectedErrorResponse { msg }) {
                Ok(_) => Ok(child),
                Err(error) => {
                    error!("Health check failed: {}", error);
                    self.spawn_child()
                }
            }
        } else {
            self.spawn_child()
        }
    }

    fn spawn_child(&self) -> Result<PythonChild, Error> {
        let mut child = PythonChild::spawn(
            &self.config.python_workspace,
            "./python_src/snippet_extractor.py",
        )?;

        let ready = child.read_message::<String, Error>()?;
        assert_eq!(ready, "ready");

        for (name, path) in &self.config.tokenizers {
            let path = path.to_str().unwrap(/* we validated this in the constructor */);
            child.send_command(&LoadTokenizer { name, path }, |msg| {
                Error::LoadingTokenizerFailed { msg }
            })?;
        }

        Ok(child)
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
