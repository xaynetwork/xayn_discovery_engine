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

use std::{net::SocketAddr, path::Path};

use figment::{
    providers::{Format, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Serialize};

pub trait Config: DeserializeOwned + Send + Sync + 'static {
    fn bind_address(&self) -> SocketAddr;
    fn log_file(&self) -> Option<&Path> {
        None
    }
}

pub(crate) fn load_config<C, U>(
    config_file: Option<&Path>,
    _update_with: U,
) -> Result<C, figment::Error>
where
    C: DeserializeOwned,
    U: Serialize,
{
    //FIXME this is just MVP for filling in the skeleton, correct implementation is in follow up PR
    Figment::new()
        .merge(Toml::file(
            config_file.unwrap_or_else(|| Path::new("config.toml")),
        ))
        .extract()
}
