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

use std::{fmt::Display, str::FromStr};

use once_cell::sync::Lazy;
use regex::Regex;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgConnectOptions, Pool, Postgres, Transaction, Type};
use thiserror::Error;

use crate::{request::TenantId, serde::serialize_redacted};

pub type Client = Pool<Postgres>;

/// Configuration for connection to postgres
#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// The default base url.
    ///
    /// Passwords in the URL will be ignored, do not set the
    /// db password with the db url.
    base_url: String,

    /// Override port from base url.
    port: Option<u16>,

    /// Override user from base url.
    user: Option<String>,

    /// Sets the password.
    #[serde(serialize_with = "serialize_redacted")]
    password: Secret<String>,

    /// Override db from base url.
    db: Option<String>,

    /// Override default application name from base url.
    application_name: Option<String>,

    /// If true skips running db migrations on start up.
    skip_migrations: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "postgres://user:pw@localhost:5432/xayn".into(),
            port: None,
            user: None,
            password: String::from("pw").into(),
            db: None,
            //TODO check how this might be used, the default we set here
            //     before didn't really work but might have been used
            application_name: None,
            skip_migrations: false,
        }
    }
}

impl Config {
    pub fn to_connection_options(&self) -> Result<PgConnectOptions, sqlx::Error> {
        let Self {
            base_url,
            port,
            user,
            password,
            db,
            application_name,
            skip_migrations: _,
        } = self;

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

/// A quoted postgres identifier.
///
/// If displayed (e.g. `.to_string()`) quotes (`"`) will be included.
///
/// This can be used for cases where a SQL query is build
/// dynamically and is parameterized over an identifier in
/// a position where postgres doesn't allow `$` bindings.
///
/// For example in `SET ROLE "role";`
///
/// Be aware that quoted identifiers are case-sensitive and limited to 63 bytes.
/// Moreover, we only allow printable us-ascii characters excluding `"`; this is stricter than [postgres](https://www.postgresql.org/docs/15/sql-syntax-lexical.html#SQL-SYNTAX-IDENTIFIERS).
#[derive(Debug, Clone, Type)]
#[sqlx(transparent)]
pub struct QuotedIdentifier(String);

impl QuotedIdentifier {
    pub fn as_unquoted_str(&self) -> &str {
        &self.0
    }

    pub fn db_name_for_tenant_id(tenant_id: TenantId) -> Self {
        format!("t:{tenant_id}").try_into()
            .unwrap(/* tenant ids are a subset of valid quoted identifiers */)
    }
}

impl FromStr for QuotedIdentifier {
    type Err = InvalidQuotedIdentifier;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.to_owned().try_into()
    }
}

impl TryFrom<String> for QuotedIdentifier {
    type Error = InvalidQuotedIdentifier;

    fn try_from(identifier: String) -> Result<Self, Self::Error> {
        static RE: Lazy<Regex> = Lazy::new(|| {
            // printable us-ascii excluding `"`
            Regex::new(r#"^[[:print:]&&[^"]]{1,63}$"#).unwrap()
        });
        if RE.is_match(&identifier) {
            Ok(Self(identifier))
        } else {
            Err(InvalidQuotedIdentifier { identifier })
        }
    }
}

impl Display for QuotedIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

#[derive(Debug, Error)]
#[error("String is not a supported quoted identifier: {identifier:?}")]
pub struct InvalidQuotedIdentifier {
    identifier: String,
}

/// Use a xact lock on given `id`.
///
/// # Warning
///
/// The lock id namespace is per-database global
/// and 64bit. This means this lock functions
/// shares the id-space with any other transaction
/// lock space.
pub async fn lock_id_until_end_of_transaction(
    tx: &'_ mut Transaction<'_, Postgres>,
    lock_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_id)
        .execute(tx)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::str;

    use super::*;

    #[test]
    fn test_quoted_identifier_parsing() {
        assert!(QuotedIdentifier::from_str("").is_err());
        assert!(QuotedIdentifier::from_str(str::from_utf8(&[0x41; 63]).unwrap()).is_ok());
        assert!(QuotedIdentifier::from_str(str::from_utf8(&[0x41; 64]).unwrap()).is_err());
        assert!(QuotedIdentifier::from_str("a").is_ok());
        for chr in ' '..='~' {
            assert_eq!(
                QuotedIdentifier::try_from(format!("{chr}")).is_ok(),
                chr != '"'
            );
        }
    }

    #[test]
    fn test_format_quoted_identifier() {
        assert_eq!(
            QuotedIdentifier::from_str("a").unwrap().to_string(),
            "\"a\""
        );
    }
}
