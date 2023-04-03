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

use std::collections::HashMap;

use anyhow::{anyhow, bail};
use futures_util::{
    future::{self, join_all},
    TryStreamExt,
};
use once_cell::sync::Lazy;
use sqlx::{migrate::Migrate, Executor};
use uuid::Uuid;
use xayn_web_api_shared::{
    elastic,
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

pub struct Config {
    postgres: postgres::Config,
    postgres_mt_user: String,
    elastic: elastic::Config,
    is_legacy: bool,
}

pub struct Silo {
    postgres: postgres::Client,
    //FIXME merge above
    elastic: elastic::Client,
    is_legacy: bool,
}

static MT_USER: Lazy<QuotedIdentifier> = Lazy::new(|| "web-api-mt".parse().unwrap());

//TODO
type Error = anyhow::Error;

impl Silo {
    /// Initializes the DB for multi-tenant usage.
    pub async fn initialize(&self) -> Result<(), Error> {
        if self.is_legacy {
            sqlx::migrate!("migrations/tenant")
                .run(&self.postgres)
                .await?;
            return Ok(());
        } else {
            sqlx::migrate!("migrations/management")
                .run(&self.postgres)
                .await?;
        }

        Ok(())
    }

    pub async fn list_tenants(&self) -> Result<Vec<TenantId>, Error> {
        if self.is_legacy {
            Ok(vec![TenantId::default()])
        } else {
            Ok(
                sqlx::query_as::<_, (TenantId,)>("SELECT tenant_id FROM management.tenants")
                    .fetch_all(&self.postgres)
                    .await?
                    .into_iter()
                    .map(|(id,)| id)
                    .collect(),
            )
        }
    }

    pub async fn create_tenant(&self) -> Result<TenantId, Error> {
        if self.is_legacy {
            bail!("can not create tenant in legacy db")
        }

        let new_id = TenantId::from(Uuid::new_v4());
        let tenant = QuotedIdentifier::db_name_for_tenant_id(new_id);

        let mut tx = self.postgres.begin().await?;

        sqlx::query("INSERT INTO management.tenants(tenant_id) VALUES (?);")
            .bind(new_id)
            .execute(&mut tx)
            .await?;

        let mt_user = &*MT_USER;
        //Hint: $ binds won't work for identifiers (e.g. schema names)
        let query = format!(
            r##"
            CREATE ROLE {tenant};
            -- do not use the AUTHORIZATION option, the tenant uses that schema but
            -- doesn't own it (tenants only own their data, not the structure it's stored in)
            GRANT {tenant} TO {mt_user};

            CREATE SCHEMA {tenant};
            GRANT USAGE ON SCHEMA {tenant} TO {tenant};

            -- make sure all object we create can be used by tenant
            -- Note:
            --   This sets the default privileges for objects created by the user running this
            --   command, this will not effect the privileges of objects created by other users.
            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT SELECT, INSERT, UPDATE, DELETE
                ON TABLES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT USAGE
                ON SEQUENCES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT EXECUTE
                ON ROUTINES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT USAGE
                ON TYPES
                TO {tenant};
        "##
        );

        tx.execute_many(query.as_str())
            .try_for_each(|_| future::ready(Ok(())))
            .await?;

        tx.commit().await?;

        Ok(new_id)
    }

    pub async fn delete_tenant(&self, tenant_id: TenantId) -> Result<(), Error> {
        if self.is_legacy {
            //FIXME error type/variant for "not multi tenant"
            bail!("can not delete tenant in legacy db")
        }

        let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
        let mut tx = self.postgres.begin().await?;

        let tenant_does_not_exist =
            sqlx::query("DELETE FROM management.tenants WHERE tenant_id = $1")
                .bind(tenant_id)
                .execute(&mut tx)
                .await?
                .rows_affected()
                == 0;

        if tenant_does_not_exist {
            return Ok(());
        }

        //Hint: $ binds won't work for identifiers (e.g. schema names)
        let query = format!("DROP ROLE {tenant}; DROP SCHEMA {tenant};");

        tx.execute_many(query.as_str())
            .try_for_each(|_| future::ready(Ok(())))
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn run_db_migration_for(&self, tenant_id: TenantId) -> Result<(), Error> {
        let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
        let mut tx = self.postgres.begin().await?;

        // set the current schema to the tenants schema for the duration
        // of the transaction, this will make migrations run in that schema
        let query = format!("SET LOCAL search_path TO {tenant};");
        tx.execute(query.as_str()).await?;

        // WARNING: Don't assume the migration runs transactional.
        //          The reason we use a transaction is to make sure
        //          that we can use SET LOCAL and use locks which automatically
        //          unlock at the end of the transaction.
        let mut migrator = sqlx::migrate!("migrations/tenant");

        // Hint: Disable using the single global lock.
        migrator.set_locking(false);

        // Hint: Lock Id is a bigint i.e. i64, so we will have some collisions but that's okay.
        let lock_id: i64 = Uuid::from(tenant_id).as_u64_pair().1 as i64;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(lock_id)
            .execute(&mut tx)
            .await?;

        migrator.run(&mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn run_all_db_migrations(
        &self,
    ) -> Result<HashMap<TenantId, Result<(), Error>>, Error> {
        //FIXME max parallel limit etc.
        let tenants = self.list_tenants().await?;
        let results = join_all(
            tenants
                .iter()
                .map(|tenant| self.run_db_migration_for(*tenant)),
        )
        .await;

        Ok(tenants.into_iter().zip(results.into_iter()).collect())
    }
}
