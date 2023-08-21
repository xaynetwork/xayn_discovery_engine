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

use std::{path::Path, sync::Mutex};

use displaydoc::Display;
use pyo3::{
    types::{PyDict, PyModule},
    Py,
    PyAny,
    PyErr,
    Python,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Snippet extraction failed: {msg}
#[derive(Debug, Display, Error)]
pub struct Error {
    msg: String,
}

impl From<PyErr> for Error {
    fn from(err: PyErr) -> Self {
        Error {
            msg: err.to_string(),
        }
    }
}

/// Configurations of the coi system.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
#[must_use]
pub struct Config {
    pub language: String,
    pub chunk_size: usize,
    pub hard_chunk_size_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "german".into(),
            chunk_size: 500,
            hard_chunk_size_limit: 520,
        }
    }
}

#[derive(Clone)]
pub struct SnippetExtractor {
    extractor: Py<PyAny>,
}

impl SnippetExtractor {
    pub fn initialize(config: &Config, tokenizer_file: &Path) -> Result<Self, Error> {
        // For some reason running this function multi-threaded leads to an import error.
        // It's not clear where the error lies but while the GIL is a global lock programs can
        // temporary suspend holding it, this means executions in two `Python::with_gil` closures
        // can internally overlap. And in case of module loading, at least for the `transformers` module
        // this seems to cause some issues making the module loading/importing fail.
        static PREVENT_MULTI_THREADED_IMPORT: Mutex<()> = Mutex::new(());

        let tokenizer_file = tokenizer_file.to_str().ok_or_else(|| Error {
            msg: "Non utf-8 tokenizer file".into(),
        })?;

        let _guard = PREVENT_MULTI_THREADED_IMPORT.lock();
        Python::with_gil(|py| {
            let src = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/python_src/extractor.py"
            ));

            let kwargs = PyDict::new(py);
            kwargs.set_item("language", &config.language)?;
            kwargs.set_item("chunk_size", config.chunk_size)?;
            kwargs.set_item("hard_chunk_size_limit", config.hard_chunk_size_limit)?;
            kwargs.set_item("tokenizer_file", tokenizer_file)?;

            let extractor = PyModule::from_code(py, src, "extractor.py", "extractor")?
                .getattr("SnippetExtractor")?
                .call((), Some(kwargs))?
                .into();

            Ok(Self { extractor })
        })
    }

    pub fn run(&self, document: &str) -> Result<Vec<String>, Error> {
        Python::with_gil(|py| {
            self.extractor
                .call_method(py, "split_text", (document,), None)?
                .extract(py)
                .map_err(Error::from)
        })
    }
}
