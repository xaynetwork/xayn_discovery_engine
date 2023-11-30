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

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::bail;
use once_cell::sync::OnceCell;

/// Tries to detect the workspace directory.
///
/// # Panics
///
/// Panics if this is not possible.
pub fn find_workspace_dir() -> &'static Path {
    static WORKSPACE: OnceCell<PathBuf> = OnceCell::new();

    WORKSPACE
        .get_or_try_init(|| {
            let output = Command::new("cargo")
                .args(["locate-project", "--workspace", "--message-format=plain"])
                .output()?;

            if !output.status.success() {
                bail!(
                    "failed to find cargo workspace: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            let Ok(path) = String::from_utf8(output.stdout) else {
                bail!("non utf8 workspace path");
            };

            let mut workspace = PathBuf::from(path.trim());

            debug_assert!(workspace.ends_with("Cargo.toml"));
            workspace.pop();

            if !workspace.is_dir() {
                bail!("workspace is not a dir: {}", workspace.display());
            }

            Ok(workspace)
        })
        .map(|path| &**path)
        .expect("failed to detect cargo workspace")
}
