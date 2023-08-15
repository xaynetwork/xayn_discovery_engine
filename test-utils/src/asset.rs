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

use cfg_if::cfg_if;

const ASSETS_DIR: &str = "assets/";

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

/// Resolves the path to the smbert model.
pub fn smbert() -> Result<PathBuf> {
    resolve_path(&[ASSETS_DIR, "smbert_v0004"])
}

/// Resolves the path to the mocked smbert model.
pub fn smbert_mocked() -> Result<PathBuf> {
    resolve_path(&[ASSETS_DIR, "smbert_mocked_v0004"])
}

/// Resolves the path to the mocked e5 model.
pub fn e5_mocked() -> Result<PathBuf> {
    resolve_path(&[ASSETS_DIR, "e5_mocked_v0000"])
}

/// Resolves the path to the xaynia model.
pub fn xaynia() -> Result<PathBuf> {
    resolve_path(&[ASSETS_DIR, "xaynia_v0002"])
}

/// Resolves the target of the onnxruntime library.
pub fn ort_target() -> Result<&'static str> {
    cfg_if! {
        if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
            Ok("linux_aarch64")
        } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
            Ok("linux_x64")
        } else if #[cfg(target_os = "macos")] {
            Ok("macos")
        } else {
            Err(Error::new(
                ErrorKind::Unsupported,
                "onnxruntime isn't available for this target os/arch",
            ))
        }
    }
}

/// Resolves the path to the onnxruntime library.
pub fn ort() -> Result<PathBuf> {
    resolve_path(&[ASSETS_DIR, "ort_v1.15.1", ort_target()?])
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
    fn test_e5_mocked() {
        assert!(e5_mocked().is_ok());
    }

    #[test]
    fn test_xaynia() {
        assert!(xaynia().is_ok());
    }

    #[test]
    fn test_ort() {
        assert!(ort().is_ok());
    }
}
