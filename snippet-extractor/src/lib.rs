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

#[derive(Clone)]
pub struct SnippetExtractor {
    extractor: Py<PyAny>,
}

impl SnippetExtractor {
    pub fn initialize() -> PyResult<Self> {
        Python::with_gil(|py| {
            let src = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/python_src/extractor.py"
            ));

            let extractor = PyModule::from_code(py, src, "extractor.py", "extractor")?
                .getattr("extract_snippets")?
                .into();

            Ok(Self { extractor })
        })
    }

    pub fn run(&self, documents: &str) -> PyResult<Vec<String>> {
        Python::with_gil(|py| {
            let kwargs = PyDict::new(py);
            kwargs.set_item("document", documents)?;
            kwargs.set_item("chunk_size", 100)?;
            kwargs.set_item("chunk_overlap", 20)?;
            self.extractor.call(py, (), Some(kwargs))?.extract(py)
        })
    }
}
