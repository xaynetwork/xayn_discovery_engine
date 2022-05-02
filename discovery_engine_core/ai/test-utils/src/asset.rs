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
    collections::HashMap,
    env::var_os,
    fs::File,
    io::{BufReader, Error, ErrorKind, Result},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use serde_json::from_reader;

pub(crate) const DATA_DIR: &str = "../discovery_engine_flutter/example/assets/";
const ASSET_MANIFEST: &str = "../discovery_engine/lib/assets/asset_manifest.json";

/// Resolves the path to the requested data relative to the workspace directory.
pub(crate) fn resolve_path(path: &[impl AsRef<Path>]) -> Result<PathBuf> {
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

#[derive(Deserialize)]
struct Asset {
    #[serde(rename(deserialize = "id"))]
    name: String,
    url_suffix: String,
}

#[derive(Deserialize)]
struct Assets {
    assets: Vec<Asset>,
}

/// Reads the asset paths from the static assets file.
fn read_assets() -> Result<HashMap<String, PathBuf>> {
    let path = resolve_path(&[ASSET_MANIFEST])?;

    from_reader::<_, Assets>(BufReader::new(File::open(path)?))
        .map(|assets| {
            assets
                .assets
                .into_iter()
                .map(|asset| (asset.name, [DATA_DIR, &asset.url_suffix].iter().collect()))
                .collect()
        })
        .map_err(|error| Error::new(ErrorKind::InvalidData, error.to_string()))
}

/// Resolves the path to the requested asset relative to the workspace directory.
pub(crate) fn resolve_asset(asset: &str) -> Result<PathBuf> {
    resolve_path(&[read_assets()?
        .get(asset)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing asset '{}'", asset)))?])
}
