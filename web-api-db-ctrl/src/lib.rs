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

use anyhow::bail;
use futures_util::{
    future::{self, join_all},
    TryStreamExt,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use sqlx::{migrate::Migrator, Acquire, Executor, Postgres, Transaction};
use tracing::error;
use uuid::Uuid;
use xayn_web_api_shared::{
    elastic,
    postgres::{self, lock_id_until_end_of_transaction, QuotedIdentifier},
    request::TenantId,
};

pub struct Config {
    postgres: postgres::Config,
    elastic: elastic::Config,
    is_legacy: bool,
}

pub struct Silo {
    postgres: postgres::Client,
    // elastic: elastic::Client,
}

static MT_USER: Lazy<QuotedIdentifier> = Lazy::new(|| "web-api-mt".parse().unwrap());

//TODO
type Error = anyhow::Error;

static MANAGEMENT_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("migrations/management");
    migrator.locking = false;
    migrator
});

static TENANT_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("migrations/tenant");
    migrator.locking = false;
    migrator
});

impl Silo {
    /// Initializes the DB for multi-tenant usage.
    ///
    /// 1. If there is a legacy tenant in public the public schema will
    ///    be renamed (and re-owned) to the `TenantId::default()` tenant.
    ///
    /// 2. Migrations to the management schema will be run (if needed).
    ///
    /// 3. In concurrently for each tenant a migration of their
    ///    schema will be run (if needed).
    pub async fn initialize(&self) -> Result<(), Error> {
        // Move out to make sure that a pool with a limit of 1 conn doesn't
        // lead to a dead lock when running tenant migrations.
        let mut conn = self.postgres.acquire().await?.detach();

        // WARNING: Many operations here might not be fully transactional.
        //          Transactions still help with scoping locks and temp.
        //          session settings.
        let mut tx = conn.begin().await?;
        // WARNING: Hardcoding this id to 0 is only okay because know exactly
        //          which ids are used when. For e.g. sqlx doing so would be a
        //          no-go hence why they derive the id from the db name.
        lock_id_until_end_of_transaction(&mut tx, 0).await?;

        let has_public_schema_tables = sqlx::query_as::<_, (i64,)>(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public';",
        )
        .fetch_one(&mut tx)
        .await?
        .0 > 0;

        if has_public_schema_tables {
            let has_migrated_legacy_schema = sqlx::query_as::<_, (i64,)>(
                "SELECT count(*)  FROM information_schema.schemata WHERE schema_name = $1;",
            )
            .fetch_one(&mut tx)
            .await?
            .0 > 0;

            if has_migrated_legacy_schema {
                bail!("database has both public legacy schemas and a migrated schema, this should be impossible");
            }

            let tenant = QuotedIdentifier::db_name_for_tenant_id(TenantId::default());
            let query = format!(
                "ALTER SCHEMA public RENAME TO {tenant};
                -- create a new public schema, wo do not grant rights to PUBLIC
                CREATE SCHEMA public;
                -- revoke privileges from public
                REVOKE ALL ON SCHEMA {tenant} FROM PUBLIC;
                -- probably unneeded but make sure it's owned by the admin user
                ALTER SCHEMA {tenant} OWNER TO CURRENT_USER;"
            );
            tx.execute_many(query.as_str())
                .try_for_each(|_| future::ready(Ok(())))
                .await?;

            create_tenant(&mut tx, TenantId::default(), true).await?;
        }

        MANAGEMENT_SCHEMA_MIGRATOR.run(&mut tx).await?;

        // We run this _before_ we release the lock but it will
        // run on concurrently on  multiple different connections.
        //
        // For this we can have the same guarantees with multi tenant as we
        // currently have with single tenant.
        //FIXME: There is a limit to how well this scales.
        let (success, failures) = self.run_all_db_migrations().await?;

        //TODO we need to decide how to handle partial failure
        if !failures.is_empty() {
            for (tenant_id, error) in failures {
                error!({ %tenant_id, %error }, "migration failed");
            }
            if success.is_empty() {
                bail!("all tenant migrations failed");
            }
        }

        tx.commit().await?;

        Ok(())
    }

    pub async fn rename_tenant(&self, tenant_id: TenantId, new_id: TenantId) -> Result<(), Error> {
        let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
        let new_name = QuotedIdentifier::db_name_for_tenant_id(new_id);

        let mut tx = self.postgres.begin().await?;

        sqlx::query("UPDATE management.tenant SET tenant_id = $1 WHERE tenant_id = $2;")
            .bind(new_id)
            .bind(tenant_id)
            .execute(&mut tx)
            .await?;

        let query = format!(
            "ALTER SCHEMA {tenant} RENAME TO {new_name};
            ALTER ROLE {tenant} RENAME TO {new_name};"
        );

        tx.execute_many(query.as_str())
            .try_for_each(|_| future::ready(Ok(())))
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_tenants(&self) -> Result<Vec<TenantId>, Error> {
        Ok(
            sqlx::query_as::<_, (TenantId,)>("SELECT tenant_id FROM management.tenant")
                .fetch_all(&self.postgres)
                .await?
                .into_iter()
                .map(|(id,)| id)
                .collect(),
        )
    }

    pub async fn create_tenant(&self) -> Result<TenantId, Error> {
        let new_id = TenantId::from(Uuid::new_v4());
        let mut tx = self.postgres.begin().await?;
        create_tenant(&mut tx, new_id, false).await?;
        tx.commit().await?;
        Ok(new_id)
    }

    pub async fn delete_tenant(&self, tenant_id: TenantId) -> Result<(), Error> {
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

        // Hint: Lock Id is a bigint i.e. i64, so we will have some collisions but that's okay.
        let lock_id: i64 = Uuid::from(tenant_id).as_u64_pair().1 as i64;
        lock_id_until_end_of_transaction(&mut tx, lock_id).await?;

        TENANT_SCHEMA_MIGRATOR.run(&mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn run_all_db_migrations(
        &self,
    ) -> Result<(Vec<TenantId>, Vec<(TenantId, Error)>), Error> {
        let tenants = self.list_tenants().await?;
        // Hint: Parallelism is implicitly limited by the connection pool.
        let results = join_all(
            tenants
                .iter()
                .map(|tenant| self.run_db_migration_for(*tenant)),
        )
        .await;

        Ok(tenants
            .into_iter()
            .zip(results.into_iter())
            .map(|(tenant, result)| match result {
                Ok(()) => Ok(tenant),
                Err(error) => Err((tenant, error)),
            })
            .partition_result())
    }
}

/// Setups up a new tenant with given id.
///
/// If `allow_existing_schema` is `true` it will
/// not fail if the schema already exists. Through
/// even if it already exist it will still setup
/// the permissions.
async fn create_tenant(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
    allow_existing_schema: bool,
) -> Result<(), Error> {
    let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
    let schema_if_not_exist = if allow_existing_schema {
        "IF NOT EXISTS"
    } else {
        ""
    };
    let mt_user = &*MT_USER;
    //Hint: $ binds won't work for identifiers (e.g. schema names)
    let query = format!(
        r##"
            CREATE ROLE {tenant};
            -- as the search_path is based on the login user this won't matter
            -- for now, but if we ever add login capabilities not having it would
            -- be a problem
            ALTER ROLE {tenant} SET search_path TO "$user";
            GRANT {tenant} TO {mt_user};

            -- do not use the AUTHORIZATION option, the tenant uses that schema but
            -- doesn't own it (tenants only own their data, not the structure it's stored in)
            CREATE SCHEMA {schema_if_not_exist} {tenant};
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

    sqlx::query("INSERT INTO management.tenants(tenant_id) VALUES (?);")
        .bind(tenant_id)
        .execute(tx)
        .await?;

    Ok(())
}
