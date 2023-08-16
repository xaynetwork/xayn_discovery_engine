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

use displaydoc::Display;
use pyo3::{
    types::{PyDict, PyModule},
    Py,
    PyAny,
    PyErr,
    PyResult,
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
    language: String,
    chunks_size: usize,
    hard_chunks_size_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "german".into(),
            chunks_size: 500,
            hard_chunks_size_limit: 520,
        }
    }
}

#[derive(Clone)]
pub struct SnippetExtractor {
    extractor: Py<PyAny>,
}

impl SnippetExtractor {
    pub fn initialize(config: Config) -> PyResult<Self> {
        Python::with_gil(|py| {
            let src = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/python_src/extractor.py"
            ));

            let kwargs = PyDict::new(py);
            kwargs.set_item("language", &config.language)?;
            kwargs.set_item("chunks_size", &config.chunks_size)?;
            kwargs.set_item("hard_chunks_size_limit", &config.hard_chunks_size_limit)?;

            let extractor = PyModule::from_code(py, src, "extractor.py", "extractor")?
                .getattr("SnippetExtractor")?
                .call((), Some(kwargs))?
                .into();

            Ok(Self { extractor })
        })
    }

    pub fn run(&self, document: &str) -> PyResult<Vec<String>> {
        Python::with_gil(|py| {
            self.extractor
                .call_method(py, "split_text", (document,), None)?
                .extract(py)
        })
    }
}
