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

use std::time::Duration;

use anyhow::bail;
use futures_util::{
    future::{self, join_all},
    TryStreamExt,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use sqlx::{migrate::Migrator, pool::PoolOptions, Connection, Executor, Postgres, Transaction};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument};
use xayn_web_api_shared::{
    postgres::{self, QuotedIdentifier},
    request::TenantId,
};

pub struct Config {
    pub postgres: postgres::Config,
    pub enable_legacy_tenant: bool,
}

#[derive(Clone)]
pub struct Silo {
    postgres: postgres::Client,
    enable_legacy_tenant: bool,
}

static MT_USER: Lazy<QuotedIdentifier> = Lazy::new(|| "web-api-mt".parse().unwrap());

//TODO
type Error = anyhow::Error;

static PUBLIC_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("migrations/public");
    migrator.locking = false;
    migrator
});

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

// WARNING: Hardcoding this id to 0 is only okay because know exactly
//          which ids are used when. For e.g. sqlx doing so would be a
//          no-go hence why they derive the id from the db name.
const MIGRATION_LOCK_ID: i64 = 0;

impl Silo {
    pub async fn builder(config: Config) -> Result<Self, Error> {
        let postgres = PoolOptions::new()
            .connect_with(config.postgres.to_connection_options()?)
            .await?;

        Ok(Self {
            postgres,
            enable_legacy_tenant: config.enable_legacy_tenant,
        })
    }

    /// Initializes the DB for multi-tenant usage.
    ///
    /// 1. If there is a legacy tenant in public the public schema will
    ///    be renamed (and re-owned) to the [`TenantId::random_legacy_tenant_id()`] tenant.
    ///
    /// 2. Migrations to the management schema will be run (if needed).
    ///
    /// 3. Concurrently for each tenant a migration of their
    ///    schema will be run (if needed).
    #[instrument(skip(self), err)]
    pub async fn initialize(&self) -> Result<Option<TenantId>, Error> {
        // Move out to make sure that a pool with a limit of 1 conn doesn't
        // lead to a dead lock when running tenant migrations. And that we
        // do release the lock in case of an error.
        let mut conn = self.postgres.acquire().await?.detach();

        lock_id_until_unlock(&mut conn, MIGRATION_LOCK_ID).await?;

        // WARNING: Many operations here might not be fully transactional.
        //          Transactions still help with scoping locks and temp.
        //          session settings.
        let mut tx = conn.begin().await?;

        info!("running management schema migration");
        run_migration_in_schema_switch_search_path(
            &mut tx,
            &"management".parse()?,
            &MANAGEMENT_SCHEMA_MIGRATOR,
        )
        .await?;

        let legacy_tenant_id = if self.enable_legacy_tenant {
            Some(self.initialize_legacy(&mut tx).await?)
        } else {
            None
        };

        info!("running public schema migration");
        run_migration_in_schema_switch_search_path(
            &mut tx,
            &"public".parse()?,
            &PUBLIC_SCHEMA_MIGRATOR,
        )
        .await?;

        tx.commit().await?;

        // We run this _before_ we release the lock but it will
        // run concurrently on multiple different connections.
        //
        // For this we can have the same guarantees with multi tenant as we
        // currently have with single tenant.
        //FIXME: There is a limit to how well this scales.
        info!("start tenant db schema migrations");
        let failures = self.run_all_db_migrations(false).await?;

        unlock_lock_id(&mut conn, MIGRATION_LOCK_ID).await?;

        //TODO we need to decide how to handle partial failure
        if !failures.is_empty() {
            for (tenant_id, error) in failures {
                error!({ %tenant_id, %error }, "migration failed");
            }
            bail!("all tenant migrations failed");
        }

        conn.close().await?;

        Ok(legacy_tenant_id)
    }

    async fn initialize_legacy(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<TenantId, Error> {
        let legacy_tenant_id = sqlx::query_as::<_, (TenantId,)>(
            "SELECT tenant_id FROM tenant WHERE is_legacy_tenant FOR UPDATE;",
        )
        .fetch_optional(&mut *tx)
        .await?;

        let legacy_tenant_id = if let Some((legacy_tenant_id,)) = legacy_tenant_id {
            legacy_tenant_id
        } else {
            let new_id = TenantId::random_legacy_tenant_id();
            create_tenant(tx, &new_id, true).await?;
            info!({tenant_id = %new_id}, "created new legacy tenant");
            new_id
        };
        Ok(legacy_tenant_id)
    }

    /// Allows using the admin user as `web-api-mt` user.
    #[instrument(skip(self), err)]
    pub async fn admin_as_mt_user_hack(&self) -> Result<(), Error> {
        info!("using the admin as mt user");
        let mt_user = &*MT_USER;
        let mut tx = self.postgres.begin().await?;

        lock_id_until_end_of_transaction(&mut tx, MIGRATION_LOCK_ID).await?;

        if create_role_if_not_exists(&mut tx, mt_user).await? {
            let query = format!(
                r#"
                ALTER USER {mt_user} SET search_path TO "$user";
                GRANT {mt_user} TO CURRENT_USER;
            "#
            );
            tx.execute(query.as_str()).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    #[instrument(skip(self), err)]
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

    #[instrument(skip(self), err)]
    pub async fn create_tenant(&self) -> Result<TenantId, Error> {
        let new_id = TenantId::random_legacy_tenant_id();
        let mut tx = self.postgres.begin().await?;
        create_tenant(&mut tx, &new_id, false).await?;
        self.run_db_migration_for(&new_id, true).await?;
        tx.commit().await?;
        Ok(new_id)
    }

    #[instrument(skip(self), err)]
    pub async fn delete_tenant(&self, tenant_id: &TenantId) -> Result<(), Error> {
        let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
        let mut tx = self.postgres.begin().await?;

        let tenant_does_not_exist =
            sqlx::query("DELETE FROM management.tenant WHERE tenant_id = $1")
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

    #[instrument(skip(self), err)]
    async fn run_db_migration_for(&self, tenant_id: &TenantId, lock_db: bool) -> Result<(), Error> {
        let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);
        let mut tx = self.postgres.begin().await?;

        // set the current schema to the tenants schema for the duration
        // of the transaction, this will make migrations run in that schema
        let query = format!("SET LOCAL search_path TO {tenant};");
        tx.execute(query.as_str()).await?;

        if lock_db {
            // Hint: Lock Id is a bigint i.e. i64, so we will have some collisions but that's okay.
            let mut lock_id: i64 = 0; //TODO Uuid::from(tenant_id).as_u64_pair().1 as i64;
            if lock_id == MIGRATION_LOCK_ID {
                lock_id += 1;
            }
            lock_id_until_end_of_transaction(&mut tx, lock_id).await?;
        }

        info!("migrate tenant {tenant}");
        TENANT_SCHEMA_MIGRATOR.run(&mut tx).await?;

        tx.commit().await?;
        Ok(())
    }

    #[instrument(skip(self), err)]
    async fn run_all_db_migrations(&self, lock_db: bool) -> Result<Vec<(TenantId, Error)>, Error> {
        let tenants = self.list_tenants().await?;
        // Hint: Parallelism is implicitly limited by the connection pool.
        let results = join_all(
            tenants
                .iter()
                .map(|tenant| self.run_db_migration_for(tenant, lock_db)),
        )
        .await;

        Ok(tenants
            .into_iter()
            .zip(results)
            .filter_map(|(tenant, result)| match result {
                Ok(()) => None,
                Err(error) => Some((tenant, error)),
            })
            .collect_vec())
    }
}

/// Sets up a new tenant with given id.
///
/// This will fail if the tenant role already exist.
///
/// This will **not** run migrations in the new tenant.
#[instrument(err)]
async fn create_tenant(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    is_legacy_tenant: bool,
) -> Result<(), Error> {
    let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);

    create_role(tx, &tenant).await?;

    let mt_user = &*MT_USER;
    //Hint: $ binds won't work for identifiers (e.g. schema names)
    let query = format!(
        r##"-- as the search_path is based on the login user this won't matter
            -- for now, but if we ever add login capabilities not having it would
            -- be a problem
            ALTER ROLE {tenant} SET search_path TO "$user";
            GRANT {tenant} TO {mt_user};"##,
    );

    tx.execute_many(query.as_str())
        .try_for_each(|_| future::ready(Ok(())))
        .await?;

    let query = if is_legacy_tenant {
        info!("moving legacy tenant from public schema to {tenant}");
        format!(
            r##"ALTER SCHEMA public RENAME TO {tenant};
                -- revoke privileges from public
                REVOKE ALL ON SCHEMA {tenant} FROM PUBLIC;
                -- probably unneeded but make sure it's owned by the admin user
                ALTER SCHEMA {tenant} OWNER TO CURRENT_USER;
                -- create a new public schema, we do not grant rights to PUBLIC
                CREATE SCHEMA public;"##
        )
    } else {
        format!(
            r##"-- do not use the AUTHORIZATION option, the tenant uses that schema but
                -- doesn't own it (tenants only own their data, not the structure it's stored in)
                CREATE SCHEMA IF NOT EXISTS {tenant};"##
        )
    };

    tx.execute_many(query.as_str())
        .try_for_each(|_| future::ready(Ok(())))
        .await?;

    let query = format!(
        r##"-- tenant is only allowed to use the schema, they don't own it
            GRANT USAGE ON SCHEMA {tenant} TO {tenant};

            -- make sure all object we create can be used by tenant
            -- Note:
            --   This sets the default privileges for objects created by the user running this
            --   command, this will not affect the privileges of objects created by other users.
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

    sqlx::query("INSERT INTO management.tenant (tenant_id, is_legacy_tenant) VALUES ($1, $2);")
        .bind(tenant_id)
        .bind(is_legacy_tenant)
        .execute(tx)
        .await?;

    Ok(())
}

/// Creates a DB role if it doesn't exist.
#[instrument(err)]
async fn create_role_if_not_exists(
    tx: &mut Transaction<'_, Postgres>,
    role: &QuotedIdentifier,
) -> Result<bool, Error> {
    let mut count = 3;
    loop {
        return if !does_role_exist(tx, role).await? {
            if let Err(err) = create_role(tx, role).await {
                count -= 1;
                if count > 0 {
                    sleep(Duration::from_millis(100)).await;
                    continue;
                } else {
                    return Err(err);
                }
            }
            Ok(true)
        } else {
            Ok(false)
        };
    }
}

#[instrument(err)]
async fn create_role(
    tx: &mut Transaction<'_, Postgres>,
    role: &QuotedIdentifier,
) -> Result<(), Error> {
    let query = format!("CREATE ROLE {role} NOINHERIT;");
    tx.execute(query.as_str()).await?;
    Ok(())
}

#[instrument(err)]
async fn does_role_exist(
    tx: &mut Transaction<'_, Postgres>,
    role: &QuotedIdentifier,
) -> Result<bool, Error> {
    Ok(
        sqlx::query("SELECT FROM pg_catalog.pg_roles WHERE rolname = $1;")
            .bind(role)
            .execute(tx)
            .await?
            .rows_affected()
            > 0,
    )
}

#[instrument(skip(migrations), err)]
async fn run_migration_in_schema_switch_search_path(
    tx: &mut Transaction<'_, Postgres>,
    schema: &QuotedIdentifier,
    migrations: &Migrator,
) -> Result<(), Error> {
    let query = format!(
        "CREATE SCHEMA IF NOT EXISTS {schema};
        SET LOCAL search_path TO {schema};"
    );
    tx.execute_many(query.as_str())
        .try_for_each(|_| future::ready(Ok(())))
        .await?;

    migrations.run(&mut *tx).await?;
    Ok(())
}

/// Use a xact lock on given `id`.
///
/// # Warning
///
/// The lock id namespace is per-database global
/// and 64bit. This means this lock functions
/// shares the id-space with any other transaction
/// lock space.
async fn lock_id_until_end_of_transaction(
    tx: &'_ mut Transaction<'_, Postgres>,
    lock_id: i64,
) -> Result<(), sqlx::Error> {
    debug!({ lock_id }, "pg_advisory_xact_lock");
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(lock_id)
        .execute(tx)
        .await?;
    Ok(())
}

/// Locks the id until it's unlocked or the pg session ends (i.e. connection dropped).
async fn lock_id_until_unlock(
    tx: impl Executor<'_, Database = Postgres>,
    lock_id: i64,
) -> Result<(), sqlx::Error> {
    debug!({ lock_id }, "pg_advisory_lock");
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(lock_id)
        .execute(tx)
        .await?;
    Ok(())
}

/// Unlocks an id locked with [`lock_id_until_unlock()`].
///
/// This *can not* be used to unlock ids locked with [`lock_id_until_end_of_transaction()`].
async fn unlock_lock_id(
    tx: impl Executor<'_, Database = Postgres>,
    lock_id: i64,
) -> Result<(), sqlx::Error> {
    let (lock_was_held,) = sqlx::query_as::<_, (bool,)>("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .fetch_one(tx)
        .await?;
    if lock_was_held {
        debug!({ lock_id }, "pg_advisory_unlock");
    } else {
        error!(
            { lock_id },
            "spurious pg_advisory_unlock which wasn't locket"
        );
    }
    Ok(())
}
