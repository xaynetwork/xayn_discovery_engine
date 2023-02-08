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

use std::fmt::Write;

/// Error which panics when creating.
///
/// Useful for using `?` in tests which will panic on error anyway.
#[derive(Debug)]
pub enum Panic {}

impl<E> From<E> for Panic
where
    E: std::error::Error,
{
    fn from(error: E) -> Self {
        let mut err_msg = format!("{error}");
        let mut next_error = error.source();
        while let Some(err) = next_error {
            write!(&mut err_msg, "\nCaused By: {err}")
                .ok(/*can't fail as we just combine strings*/);
            next_error = err.source();
        }
        panic!("{err_msg}");
    }
}
