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

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
};

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub trait Config: DeserializeOwned + Send + Sync + 'static {
    fn bind_address(&self) -> SocketAddr;
    fn log_file(&self) -> Option<&Path> {
        None
    }
}

#[derive(Deserialize, Debug)]
pub struct CommonConfig {
    #[serde(default = "default_bind_address")]
    bind_to: SocketAddr,

    #[serde(default)]
    log_file: Option<PathBuf>,
}

fn default_bind_address() -> SocketAddr {
    SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080).into()
}

impl Config for CommonConfig {
    fn bind_address(&self) -> SocketAddr {
        self.bind_to
    }
    fn log_file(&self) -> Option<&Path> {
        self.log_file.as_deref()
    }
}

/// Load the configuration into given type.
///
/// # Load order/priority
///
/// This will by ascending priority load:
///
/// 1. `./config.toml` or specified toml config file
/// 2. `./.env`
/// 3. `./.env.local`
/// 4. process environment
/// 5. options passed through `update_with`
///
/// Config values loaded from higher priority sources override such from lower
/// priority sources. E.g. values current process environment overrides values
/// from any other source.
///
/// `.env` is included to avoid confusion of env variables missing when calling
/// cargo directly instead of indirectly through `just`.
///
/// `.env.local` is a semi-standard way to add temporary local overrides you
/// don't want to commit.
///
/// # Env and .env
///
/// Environment variables from `.env` and `.env.local` will be loaded in the process
/// environment if they don't already exist there (keeping priority as described above).
///
/// For creating the config type instance only environment variables with the
/// `XAYN_WEB_API_` prefix will be considered and the prefix is stripped.
///
/// Env variables are split by `__`. I.e. `XAYN_WEB_API_FOO__BAR=12` will be treated like
/// the json `{ "foo": { "bar": 12 } }` wrt. deserializing the config. The `__` is needed as
/// else there wouldn't be a way to have fields like `bind_to` in the config.
pub fn load_config<C, U>(config_file: Option<&Path>, update_with: U) -> Result<C, figment::Error>
where
    C: DeserializeOwned,
    U: Serialize,
{
    // the order must be from highest to lowest priority
    // or else it won't work correctly
    //FIXME figment Provider for .env, but it's annoying due to side effects
    load_dotenv(".env.local")?;
    load_dotenv(".env")?;

    Figment::new()
        .join(Serialized::globals(update_with))
        .join(Env::prefixed("XAYN_WEB_API_").split("__"))
        .join(Toml::file(config_file.unwrap_or(Path::new("config.toml"))))
        .extract()
        .map_err(Into::into)
}

fn load_dotenv(file_name: &str) -> Result<(), figment::Error> {
    match dotenvy::from_filename(file_name) {
        Err(error) if !error.not_found() => {
            Err(figment::Error::from(error.to_string()).with_path(file_name))
        }
        _ => Ok(()),
    }
}
