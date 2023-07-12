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

use std::{
    env::var_os,
    io::{Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

const DATA_DIR: &str = "assets/";

/// Resolves the path to the requested data relative to the workspace directory.
fn resolve_path(path: &[impl AsRef<Path>]) -> Result<PathBuf> {
    let manifest = var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "missing CARGO_MANIFEST_DIR"))?;

    let workspace = PathBuf::from(manifest)
        .ancestors()
        .find(|path| path.to_path_buf().join("Cargo.lock").exists())
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "missing cargo workspace dir"))?
        .to_path_buf();

    path.iter()
        .fold(workspace, |path, component| path.join(component))
        .canonicalize()
}

/// Resolves the path to the qasmbert.
pub fn qasmbert() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, "qasmbert_v0002"])
}

/// Resolves the path to the smbert.
pub fn smbert() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, "smbert_v0004"])
}

/// Resolves the path to the mocked smbert.
pub fn smbert_mocked() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, "smbert_mocked_v0004"])
}

/// Resolves the path to the e5 like model.
pub fn e5_mocked() -> Result<PathBuf> {
    resolve_path(&[DATA_DIR, "e5_mocked_v0000"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smbert() {
        assert!(smbert().is_ok());
    }

    #[test]
    fn test_smbert_mocked() {
        assert!(smbert_mocked().is_ok());
    }

    #[test]
    fn test_qasmbert() {
        assert!(qasmbert().is_ok());
    }

    #[test]
    fn test_e5_mocked() {
        assert!(e5_mocked().is_ok());
    }
}
