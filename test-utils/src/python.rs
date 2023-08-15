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
    env,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Once,
};

use anyhow::{bail, Error};
use pyo3::{types::PyDict, Python};

/// Initializes pyo3 and inject venv.
///
/// # Panics
///
/// As this is a test util it will panic on failure.
pub fn initialize_python() {
    pyo3::prepare_freethreaded_python();

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        Python::with_gil(inject_snippet_extractor_venv_activation).unwrap();
    })
}

/// Inject the snippet-extractor venv into the given python instance.
pub fn inject_snippet_extractor_venv_activation(py: Python<'_>) -> Result<(), Error> {
    let mut py_project_dir = env::current_dir()?;

    if !py_project_dir.join("Pipfile").exists() {
        let mut workspace = cmd_returning_str_path(
            "detect workspace",
            "cargo",
            &["locate-project", "--workspace", "--message-format=plain"],
            &py_project_dir,
        )?;

        if workspace.is_file() {
            workspace.pop();
        }

        py_project_dir = workspace.join("snippet-extractor");

        if !py_project_dir.join("Pipfile").exists() {
            bail!(
                "can not find python directory, tried: {}",
                py_project_dir.display()
            );
        }
    }

    let activation_script_path =
        cmd_returning_str_path("detect venv", "pipenv", &["--venv"], &py_project_dir)?
            .join("bin")
            .join("activate_this.py");
    let activation_script_path = activation_script_path
        .to_str()
        .unwrap(/*we created it from a string*/);

    if !Path::new(activation_script_path).exists() {
        bail!("venv activation script doesn't exist: {activation_script_path}");
    }

    let activation_script = fs::read(&activation_script_path)?;
    let Ok(activation_script) = String::from_utf8(activation_script) else {
        bail!("activation script contained non-utf8 characters: {activation_script_path}");
    };

    let globals = PyDict::new(py);
    // We need to set the `__file__` variable as described in the activation script.
    globals.set_item("__file__", activation_script_path)?;
    py.run(&activation_script, Some(globals), None)?;

    Ok(())
}

fn cmd_returning_str_path(
    hint: &'static str,
    cmd: &str,
    args: &[&str],
    cwd: &Path,
) -> Result<PathBuf, Error> {
    let output = Command::new(cmd).args(args).current_dir(cwd).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("failed to {hint}: {stderr}");
    }

    let Ok(path) = String::from_utf8(output.stdout) else {
        bail!("non utf8 {hint} path");
    };
    let path = Path::new(path.trim());
    if !path.exists() {
        bail!("{hint} path doesn't exist: {}", path.display());
    }

    Ok(path.to_owned())
}
