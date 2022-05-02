// Copyright 2021 Xayn AG
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

//! Path resolvers for the `SMBert` assets.

use std::{io::Result, path::PathBuf};

use crate::asset::resolve_asset;

/// Resolves the path to the `SMBert` vocabulary.
pub fn vocab() -> Result<PathBuf> {
    resolve_asset("smbertVocab")
}

/// Resolves the path to the `SMBert` model.
pub fn model() -> Result<PathBuf> {
    resolve_asset("smbertModel")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocab() {
        assert!(vocab().is_ok());
    }

    #[test]
    fn test_model() {
        assert!(model().is_ok());
    }
}
