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

//! Path resolvers for the `KPE` assets.

use std::{io::Result, path::PathBuf};

use crate::asset::{resolve_path, DATA_DIR};

const ASSET: &str = "kpe_v0001";

/// Resolves the path to the Bert vocabulary.
pub fn vocab() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, ASSET, "vocab.txt"])
}

/// Resolves the path to the Bert model.
pub fn bert() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, ASSET, "bert-quantized.onnx"])
}

/// Resolves the path to the CNN model.
pub fn cnn() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, ASSET, "cnn.binparams"])
}

/// Resolves the path to the Classifier model.
pub fn classifier() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, ASSET, "classifier.binparams"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vocab() {
        assert!(vocab().is_ok());
    }

    #[test]
    fn test_bert() {
        assert!(bert().is_ok());
    }

    #[test]
    fn test_cnn() {
        assert!(cnn().is_ok());
    }

    #[test]
    fn test_classifier() {
        assert!(classifier().is_ok());
    }
}
