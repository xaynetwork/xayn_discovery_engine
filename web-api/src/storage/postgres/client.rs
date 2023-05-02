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

use async_stream::try_stream;
use either::Either;
use futures_util::{future::BoxFuture, stream::BoxStream, FutureExt, StreamExt, TryStreamExt};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::{PoolConnection, PoolOptions},
    postgres::{PgConnectOptions, PgQueryResult, PgRow, PgStatement, PgTypeInfo},
    Acquire,
    Describe,
    Execute,
    Executor,
    Pool,
    Postgres,
    Transaction,
};
use tracing::{info, instrument};

use super::utils::{InvalidQuotedIdentifier, QuotedIdentifier};
use crate::{
    error::common::InternalError,
    middleware::request_context::TenantId,
    utils::serialize_redacted,
    Error,
    SetupError,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Config {
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

    /// Number of connections in the pool.
    #[serde(default = "default_min_pool_size")]
    min_pool_size: u8,
}

fn default_min_pool_size() -> u8 {
    25
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "postgres://user:pw@localhost:5432/xayn".into(),
            port: None,
            user: None,
            password: String::from("pw").into(),
            db: None,
            application_name: option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}")),
            skip_migrations: false,
            min_pool_size: default_min_pool_size(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DatabaseBuilder {
    pool: Pool<Postgres>,
}

impl DatabaseBuilder {
    pub(crate) async fn close(&self) {
        self.pool.close().await;
    }

    pub(crate) fn build_for(&self, tenant_id: &TenantId) -> Result<Database, Error> {
        Ok(Database {
            pool: self.pool.clone(),
            tenant_db_name: db_name_for_tenant_id(tenant_id).map_err(InternalError::from_std)?,
        })
    }
}

fn db_name_for_tenant_id(
    tenant_id: &TenantId,
) -> Result<QuotedIdentifier, InvalidQuotedIdentifier> {
    format!("t:{tenant_id}").try_into()
}

#[derive(Debug)]
pub(crate) struct Database {
    pool: Pool<Postgres>,
    #[allow(dead_code)]
    tenant_db_name: QuotedIdentifier,
}

impl Database {
    #[instrument]
    pub(crate) async fn builder(
        config: &Config,
        enable_legacy_tenant: bool,
    ) -> Result<DatabaseBuilder, SetupError> {
        let options = Self::build_connection_options(config)?;
        info!("starting postgres setup");
        let pool = PoolOptions::new()
            .min_connections(u32::from(config.min_pool_size))
            .after_release(|conn, _metadata| {
                async {
                    sqlx::query("RESET ROLE;").execute(conn).await?;
                    Ok(true)
                }
                .boxed()
            })
            .connect_with(options)
            .await?;

        if !config.skip_migrations {
            sqlx::migrate!().run(&pool).await?;

            //FIXME handle legacy tenant here (in follow up PR)
            let _ = enable_legacy_tenant;
        }

        Ok(DatabaseBuilder { pool })
    }

    fn build_connection_options(config: &Config) -> Result<PgConnectOptions, sqlx::Error> {
        let Config {
            base_url,
            port,
            user,
            password,
            db,
            application_name,
            ..
        } = config;

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

    async fn set_role(
        &self,
        _conn: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        // prepare/bind doesn't work with `SET ROLE` so we need to do a bit
        // of encoding/safety checking ourself
        //FIXME to avoid problems this is commented out until follow up PRs
        //      which properly setup the database
        // sqlx::query(&format!("SET ROLE {};", self.tenant_db_name))
        //     .execute(conn)
        //     .await
        //     .map(|_| ())
        #![allow(clippy::unused_async)]
        Ok(())
    }

    pub(crate) async fn acquire(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        info!("db_conn=acquiring");
        let mut conn = self.pool.acquire().await?;
        info!("db_conn=acquired");
        self.set_role(&mut conn).await?;
        info!("db_conn=ready");
        Ok(conn)
    }

    pub(crate) async fn begin(&self) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        let mut conn = self.pool.begin().await?;
        self.set_role(&mut conn).await?;
        Ok(conn)
    }
}

impl<'c> Acquire<'c> for &'c Database {
    type Database = Postgres;

    type Connection = PoolConnection<Postgres>;

    fn acquire(self) -> BoxFuture<'c, Result<Self::Connection, sqlx::Error>> {
        self.acquire().boxed()
    }

    fn begin(self) -> BoxFuture<'c, Result<Transaction<'c, Self::Database>, sqlx::Error>> {
        self.begin().boxed()
    }
}

impl<'c> Executor<'c> for &'c Database {
    type Database = Postgres;

    fn fetch_many<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxStream<'e, Result<Either<PgQueryResult, PgRow>, sqlx::Error>>
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
    {
        try_stream! {
            let mut conn = self.acquire().await?;
            let mut results = conn.fetch_many(query);
            while let Some(item) = results.try_next().await? {
                yield item;
            }
        }
        .boxed()
    }

    fn fetch_optional<'e, 'q: 'e, E: 'q>(
        self,
        query: E,
    ) -> BoxFuture<'e, Result<Option<PgRow>, sqlx::Error>>
    where
        'c: 'e,
        E: Execute<'q, Self::Database>,
    {
        async { self.acquire().await?.fetch_optional(query).await }.boxed()
    }

    fn prepare_with<'e, 'q: 'e>(
        self,
        sql: &'q str,
        parameters: &'e [PgTypeInfo],
    ) -> BoxFuture<'e, Result<PgStatement<'q>, sqlx::Error>>
    where
        'c: 'e,
    {
        async { self.acquire().await?.prepare_with(sql, parameters).await }.boxed()
    }

    fn describe<'e, 'q: 'e>(
        self,
        sql: &'q str,
    ) -> BoxFuture<'e, Result<Describe<Self::Database>, sqlx::Error>>
    where
        'c: 'e,
    {
        async { self.acquire().await?.describe(sql).await }.boxed()
    }
}
