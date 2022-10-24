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

use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::{pool::PoolOptions, postgres::PgConnectOptions, Pool, Postgres};

#[derive(Deserialize, Debug)]
pub struct Config {
    /// The default base url.
    #[serde(default = "default_base_url")]
    base_url: String,

    /// Override port from base url.
    #[serde(default)]
    port: Option<u16>,

    /// Override user from base url.
    #[serde(default)]
    user: Option<String>,

    /// Override password from base url.
    #[serde(default)]
    password: Option<Secret<String>>,

    /// Override db from base url.
    #[serde(default)]
    db: Option<String>,

    /// Override default application name from base url.
    ///
    /// Defaults to `xayn-web-{CARGO_BIN_NAME}`.
    #[serde(default = "default_application_name")]
    application_name: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            user: None,
            password: None,
            db: None,
            port: None,
            application_name: default_application_name(),
        }
    }
}

fn default_base_url() -> String {
    "postgres://user:pw@localhost:5432/xayn".into()
}

fn default_application_name() -> Option<String> {
    option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}"))
}

impl Config {
    pub(crate) async fn create_connection_pool(&self) -> Result<Pool<Postgres>, sqlx::Error> {
        let options = self.create_connection_options()?;
        PoolOptions::new().connect_with(options).await
    }

    fn create_connection_options(&self) -> Result<PgConnectOptions, sqlx::Error> {
        let Self {
            base_url,
            port,
            user,
            password,
            db,
            application_name,
        } = &self;

        let mut options: PgConnectOptions = base_url.parse()?;

        if let Some(user) = user {
            options = options.username(user);
        }
        if let Some(port) = port {
            options = options.port(*port);
        }
        if let Some(password) = password {
            options = options.password(password.expose_secret())
        }
        if let Some(db) = db {
            options = options.database(db);
        }
        if let Some(application_name) = application_name {
            options = options.application_name(application_name);
        }

        Ok(options)
    }
}
