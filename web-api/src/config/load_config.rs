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

use std::{fmt::Display, path::Path};

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Serialize};

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
/// priority sources. E.g. values defined in `update_with` override values
/// from any other source.
///
/// `.env` is included to avoid confusion with env variables missing when calling
/// cargo directly instead of indirectly through `just`.
///
/// `.env.local` is a semi-standard way to add temporary local overrides that you
/// don't want to commit.
///
/// # Env and .env
///
/// Environment variables from `.env` and `.env.local` will be loaded into the process
/// environment if they don't already exist there (keeping priority as described above).
///
/// When creating the config type instance, only environment variables which start with
/// one of the names passed in `application_names` will be considered (case insensitive).
///
/// Variables with names earlier in the array take priority over variables with names
/// later in the array.
///
/// Env variable are converted into a config path by splitting it at `__` (and stripping
/// the application name). E.g. `XAYN_WEB_API__FOO__BAR=12` will be treated like
/// the json `{ "foo": { "bar": 12 } }` wrt. deserializing the config if `XAYN_WEB_API` is
/// in `application_names`.
pub(crate) fn load_config<C, U>(
    application_names: impl IntoIterator<Item = impl Display>,
    config: Option<&str>,
    update_with: U,
) -> Result<C, figment::Error>
where
    C: DeserializeOwned,
    U: Serialize,
{
    // the order must be from highest to lowest priority
    // or else it won't work correctly
    //FIXME figment Provider for .env, but it's annoying due to side effects
    load_dotenv(".env.local")?;
    load_dotenv(".env")?;

    let mut figment = Figment::new().join(Serialized::defaults(update_with));

    for name in application_names {
        figment = figment.join(Env::prefixed(&format!("{name}__")).split("__"));
    }

    let provider = if let Some(content_or_path) = config {
        if let Some(content) = content_or_path.strip_prefix("inline:") {
            Toml::string(content)
        } else {
            let path = Path::new(content_or_path);
            if path.is_file() {
                Toml::file(path)
            } else {
                return Err(figment::Error::from(format!(
                    "Config file missing or not a file: {}",
                    path.display()
                )));
            }
        }
    } else {
        // Be aware that this will look in the current dir and if
        // it doesn't find the config there also searches parent
        // directories.
        //
        // Furthermore it will NOT fail if no config is found.
        Toml::file("config.toml")
    };

    figment.join(provider).extract().map_err(Into::into)
}

fn load_dotenv(file_name: &str) -> Result<(), figment::Error> {
    match dotenvy::from_filename(file_name) {
        Err(error) if !error.not_found() => {
            Err(figment::Error::from(error.to_string()).with_path(file_name))
        }
        _ => Ok(()),
    }
}
