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

use std::{borrow::Cow, error::Error, fmt::Display, path::Path};

use tracing::error;

/// Like [`tokio::fs::remove_file()`] but doesn't fail if the file doesn't exist.
pub(crate) async fn remove_file_if_exists(
    path: impl AsRef<Path> + Send,
) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(path).await {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
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

    /// Like [`std::result::Result::ok()`] and the error is moved into the passed in vec (using `push`).
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
                    *slot = Some(err);
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

/// Minimalists implementation of an compound error consisting of a message an a list of sub-errors.
#[derive(Debug)]
pub(crate) struct CompoundError<E: Error + 'static> {
    msg: Cow<'static, str>,
    errors: Vec<E>,
}

impl<E: Error + 'static> CompoundError<E> {
    pub(crate) fn new(msg: impl Into<Cow<'static, str>>, errors: Vec<E>) -> Self {
        Self {
            msg: msg.into(),
            errors,
        }
    }
}

impl<E: Error + 'static> Display for CompoundError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)?;
        writeln!(f, "({} errors):", self.errors.len())?;
        for (idx, error) in self.errors.iter().enumerate() {
            writeln!(f, "{idx}: {error}")?;
        }
        Ok(())
    }
}

impl<E: Error + 'static> Error for CompoundError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // we don't use source, so this is good enough
        self.errors.first().map(|err| err as _)
    }
}
