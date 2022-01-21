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
    env,
    fs::read_dir,
    path::{Path, PathBuf},
};

use cbindgen::Config;
use heck::ToUpperCamelCase;

fn main() {
    let crate_name = env::var("CARGO_PKG_NAME").unwrap();
    let crate_name_camel = crate_name.to_upper_camel_case();
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let manifest_path = manifest_dir.join("Cargo.toml");
    let source_dir = manifest_dir.join("src");

    let config_file = manifest_dir.join("cbindgen.toml");
    let header_out_file = manifest_dir
        .join("include")
        .join(crate_name_camel)
        .with_extension("h");

    cargo_rerun_if_changed(&manifest_path);
    cargo_rerun_if_changed(&config_file);
    cargo_rerun_if_changed(&header_out_file);
    cargo_rerun_if_changed(&source_dir);

    let config = Config::from_file(config_file).expect("Failed to read config.");
    cbindgen::generate_with_config(manifest_dir, config)
        .expect("Failed to generate bindings.")
        .write_to_file(header_out_file);
}

// cargo doesn't check directories recursively so we have to do it by hand, also emitting a
// rerun-if line cancels the default rerun for changes in the crate directory
fn cargo_rerun_if_changed(entry: impl AsRef<Path>) {
    let entry = entry.as_ref();
    if entry.is_dir() {
        for entry in read_dir(entry).expect("Failed to read dir.") {
            cargo_rerun_if_changed(entry.expect("Failed to read entry.").path());
        }
    } else {
        println!("cargo:rerun-if-changed={}", entry.display());
    }
}
