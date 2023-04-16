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
use sqlx::{
    pool::{PoolConnection, PoolOptions},
    postgres::{PgQueryResult, PgRow, PgStatement, PgTypeInfo},
    Acquire,
    Describe,
    Execute,
    Executor,
    Pool,
    Postgres,
    Transaction,
};
use tracing::{info, instrument};
use xayn_web_api_db_ctrl::Silo;
use xayn_web_api_shared::{
    postgres::{Config, QuotedIdentifier},
    request::TenantId,
};

use crate::{tenants, SetupError};

#[derive(Clone)]
pub(crate) struct DatabaseBuilder {
    pool: Pool<Postgres>,
    legacy_tenant: Option<TenantId>,
}

impl DatabaseBuilder {
    pub(crate) async fn close(&self) {
        self.pool.close().await;
    }

    pub(crate) fn build_for(&self, tenant_id: &TenantId) -> Database {
        Database {
            pool: self.pool.clone(),
            tenant_db_name: QuotedIdentifier::db_name_for_tenant_id(tenant_id),
        }
    }

    pub(crate) fn legacy_tenant(&self) -> Option<TenantId> {
        self.legacy_tenant
    }
}

#[derive(Debug)]
pub(crate) struct Database {
    pool: Pool<Postgres>,
    #[allow(dead_code)]
    tenant_db_name: QuotedIdentifier,
}

impl Database {
    #[instrument(skip(config), err)]
    pub(crate) async fn builder(
        config: &Config,
        tenants: &tenants::Config,
    ) -> Result<DatabaseBuilder, SetupError> {
        let silo = Silo::builder(xayn_web_api_db_ctrl::Config {
            postgres: config.clone(),
            enable_legacy_tenant: tenants.enable_legacy_tenant,
        })
        .await?;

        // FIXME: remove this once we have a proper separation between
        //        a admin pg user owning the db structure and a web-api-mt
        //        user which can only use tables but nothing more.
        silo.admin_as_mt_user_hack().await?;

        // FIXME: long term this should be run by the control plane,
        //        in a different binary/lambda or similar before we
        //        start updating the instances.
        let legacy_tenant = silo.initialize().await?;

        let options = config.to_connection_options()?;
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

        Ok(DatabaseBuilder {
            pool,
            legacy_tenant,
        })
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
