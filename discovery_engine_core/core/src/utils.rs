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

//! Collection of utility functions.

use std::{fmt::Display, io, path::Path};

use tracing::error;

/// Like [`tokio::fs::remove_file()`] but doesn't fail if the file doesn't exist.
#[cfg(feature = "storage")]
pub(crate) async fn remove_file_if_exists(path: &Path) -> Result<(), io::Error> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

pub(crate) trait MiscErrorExt<T, E> {
    // Logs the contained error if there is one.
    fn log_error(self) -> Self
    where
        E: Display;

    /// Like [`std::result::Result::ok()`] but if `slot` is `None` the error is moved into `slot`.
    fn extract_first_error(self, slot: &mut Option<E>) -> Option<T>;

    /// Like [`std::result::Result::ok()`] the error is moved into the passed in vec (using `push`).
    fn extract_error(self, push_to: &mut Vec<E>) -> Option<T>;
}

impl<T, E> MiscErrorExt<T, E> for Result<T, E> {
    fn log_error(self) -> Self
    where
        E: Display,
    {
        if let Err(err) = &self {
            error!(error = %err);
        }
        self
    }

    fn extract_first_error(self, slot: &mut Option<E>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(err) => {
                if slot.is_none() {
                    *slot = Some(err)
                }
                None
            }
        }
    }

    fn extract_error(self, push_to: &mut Vec<E>) -> Option<T> {
        match self {
            Ok(val) => Some(val),
            Err(err) => {
                push_to.push(err);
                None
            }
        }
    }
}
