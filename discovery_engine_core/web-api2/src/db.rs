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
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolOptions, postgres::PgConnectOptions, Pool, Postgres, QueryBuilder};

use crate::{models::DocumentId, utils::serialize_redacted, Error};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// The default base url.
    ///
    /// Passwords in the URL will be ignored, do not set the
    /// db password with the db url.
    #[serde(default = "default_base_url")]
    base_url: String,

    /// Override port from base url.
    #[serde(default)]
    port: Option<u16>,

    /// Override user from base url.
    #[serde(default)]
    user: Option<String>,

    /// Sets the password.
    #[serde(default = "default_password", serialize_with = "serialize_redacted")]
    password: Secret<String>,

    /// Override db from base url.
    #[serde(default)]
    db: Option<String>,

    /// Override default application name from base url.
    ///
    /// Defaults to `xayn-web-{CARGO_BIN_NAME}`.
    #[serde(default = "default_application_name")]
    application_name: Option<String>,

    /// If true skips running db migrations on start up.
    #[serde(default)]
    skip_migrations: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            user: None,
            password: default_password(),
            db: None,
            port: None,
            application_name: default_application_name(),
            skip_migrations: false,
        }
    }
}

fn default_password() -> Secret<String> {
    String::from("pw").into()
}

fn default_base_url() -> String {
    "postgres://user:pw@localhost:5432/xayn".into()
}

fn default_application_name() -> Option<String> {
    option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}"))
}

impl Config {
    pub(crate) async fn setup_database(&self) -> Result<Database, sqlx::Error> {
        let options = self.build_connection_options()?;
        let pool = PoolOptions::new().connect_with(options).await?;
        if !self.skip_migrations {
            sqlx::migrate!().run(&pool).await?;
        }
        Ok(Database { pool })
    }

    fn build_connection_options(&self) -> Result<PgConnectOptions, sqlx::Error> {
        let Self {
            base_url,
            port,
            user,
            password,
            db,
            application_name,
            skip_migrations: _,
        } = &self;

        let mut options = base_url
            .parse::<PgConnectOptions>()?
            .password(password.expose_secret());

        if let Some(user) = user {
            options = options.username(user);
        }
        if let Some(port) = port {
            options = options.port(*port);
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

pub(crate) struct Database {
    #[allow(dead_code)]
    pool: Pool<Postgres>,
}

impl Database {
    pub(crate) async fn delete_documents(&self, documents: &[DocumentId]) -> Result<(), Error> {
        QueryBuilder::new("DELETE FROM interaction WHERE doc_id in")
            .push_tuples(documents, |mut query, id| {
                query.push(id);
            })
            .build()
            .persistent(false)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
