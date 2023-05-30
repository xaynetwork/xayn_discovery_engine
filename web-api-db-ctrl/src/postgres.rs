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

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use anyhow::anyhow;
use futures_util::{
    future::{self, join_all},
    Future,
    TryStreamExt,
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use sqlx::{migrate::Migrator, Connection, Executor, Pool, Postgres, Transaction};
use tokio::time::sleep;
use tracing::{debug, error, info, instrument};
use xayn_web_api_shared::{postgres::QuotedIdentifier, request::TenantId};

use crate::{Error, Tenant};

static MT_USER: Lazy<QuotedIdentifier> = Lazy::new(|| "web-api-mt".parse().unwrap());

static PUBLIC_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("postgres/public");
    migrator.locking = false;
    migrator
});

static MANAGEMENT_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("postgres/management");
    migrator.locking = false;
    migrator
});

static TENANT_SCHEMA_MIGRATOR: Lazy<Migrator> = Lazy::new(|| {
    let mut migrator = sqlx::migrate!("postgres/tenant");
    migrator.locking = false;
    migrator
});

// WARNING: Hardcoding this id to 0 is only okay because we know exactly
//          which ids are used when. For e.g. sqlx doing so would be a
//          no-go hence why they derive the id from the db name.
const MIGRATION_LOCK_ID: i64 = 0;

/// Initializes the DB for multi-tenant usage.
///
/// 1. If there is a legacy tenant in public the public schema will
///    be renamed (and re-owned) to the [`TenantId::random_legacy_tenant_id()`] tenant.
///
/// 2. Migrations to the management schema will be run (if needed).
///
/// 3. Concurrently for each tenant a migration of their
///    schema will be run (if needed).
#[instrument(skip_all, err)] //TODO log enable_legacy_tenant = legacy_setup.is_some
pub(super) async fn initialize<F1, F2>(
    pool: &Pool<Postgres>,
    legacy_setup: Option<impl FnOnce(TenantId) -> F1>,
    migrate_tenant: impl Fn(TenantId) -> F2,
) -> Result<Option<TenantId>, Error>
where
    F1: Future<Output = Result<(), Error>>,
    F2: Future<Output = Result<(), Error>>,
{
    // Move out to make sure that a pool with a limit of 1 conn doesn't
    // lead to a dead lock when running tenant migrations. And that we
    // do release the lock in case of an error.
    let mut conn = pool.acquire().await?.detach();

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

    let legacy_tenant_id = if let Some(legacy_setup) = legacy_setup {
        Some(initialize_legacy(&mut tx, legacy_setup).await?)
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
    let failures = run_all_db_migrations(pool, false, migrate_tenant).await?;

    unlock_lock_id(&mut conn, MIGRATION_LOCK_ID).await?;

    for (tenant_id, error) in &failures {
        error!({ %tenant_id, %error }, "migration failed");
    }

    conn.close().await?;

    //TODO we need to decide how to handle partial failure
    if failures.is_empty() {
        Ok(legacy_tenant_id)
    } else {
        Err(anyhow!("some tenant migrations failed"))
    }
}

async fn initialize_legacy<F>(
    tx: &mut Transaction<'_, Postgres>,
    legacy_setup: impl FnOnce(TenantId) -> F,
) -> Result<TenantId, Error>
where
    F: Future<Output = Result<(), Error>>,
{
    let legacy_tenant_id = sqlx::query_as::<_, (TenantId,)>(
        "SELECT tenant_id FROM tenant WHERE is_legacy_tenant FOR UPDATE;",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let legacy_tenant_id = if let Some((legacy_tenant_id,)) = legacy_tenant_id {
        legacy_tenant_id
    } else {
        let new_id = TenantId::random_legacy_tenant_id();
        create_tenant_role_and_schema(tx, &new_id, LegacyHint::MigrateSchema).await?;
        legacy_setup(new_id.clone()).await?;
        info!({tenant_id = %new_id}, "created new legacy tenant");
        new_id
    };
    Ok(legacy_tenant_id)
}

#[instrument(skip(pool, migrate_tenant), err)]
async fn run_all_db_migrations<F>(
    pool: &Pool<Postgres>,
    lock_db: bool,
    migrate_tenant: impl Fn(TenantId) -> F,
) -> Result<Vec<(TenantId, Error)>, Error>
where
    F: Future<Output = Result<(), Error>>,
{
    let tenants = list_tenants(pool).await?;
    // Hint: Parallelism is implicitly limited by the connection pool.
    let results = join_all(tenants.iter().map(|tenant| {
        let migrate_tenant = &migrate_tenant;
        async move {
            let mut tx = pool.begin().await?;
            run_db_migration_for(&mut tx, &tenant.tenant_id, lock_db).await?;
            migrate_tenant(tenant.tenant_id.clone()).await?;
            tx.commit().await?;
            Ok(())
        }
    }))
    .await;

    Ok(tenants
        .into_iter()
        .zip(results)
        .filter_map(|(tenant, result)| match result {
            Ok(()) => None,
            Err(error) => Some((tenant.tenant_id, error)),
        })
        .collect_vec())
}

#[instrument(skip(tx), err)]
async fn run_db_migration_for(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    lock_db: bool,
) -> Result<(), Error> {
    let tenant = QuotedIdentifier::db_name_for_tenant_id(tenant_id);

    // set the current schema to the tenants schema for the duration
    // of the transaction, this will make migrations run in that schema
    let query = format!("SET LOCAL search_path TO {tenant};");
    tx.execute(query.as_str()).await?;

    if lock_db {
        let lock_id = generate_tenant_lock_id(tenant_id);
        lock_id_until_end_of_transaction(tx, lock_id).await?;
    }

    info!("migrate tenant {tenant}");
    TENANT_SCHEMA_MIGRATOR.run(tx).await?;

    Ok(())
}

/// Allows using the admin user as `web-api-mt` user.
#[instrument(skip(pool), err)]
pub(super) async fn admin_as_mt_user_hack(pool: &Pool<Postgres>) -> Result<(), Error> {
    info!("using the admin as mt user");
    let mt_user = &*MT_USER;
    let mut tx = pool.begin().await?;

    lock_id_until_end_of_transaction(&mut tx, MIGRATION_LOCK_ID).await?;

    create_role_if_not_exists(pool, mt_user, |mut tx| async move {
        let query = format!(
            r#"
            ALTER USER {mt_user} SET search_path TO "$user";
            GRANT {mt_user} TO CURRENT_USER;
        "#
        );
        tx.execute(query.as_str()).await?;
        tx.commit().await?;
        Ok(())
    })
    .await?;

    tx.commit().await?;
    Ok(())
}

#[instrument(skip(pool), err)]
pub(super) async fn list_tenants(pool: &Pool<Postgres>) -> Result<Vec<Tenant>, Error> {
    Ok(sqlx::query_as::<_, (TenantId, bool)>(
        "SELECT tenant_id, is_legacy_tenant FROM management.tenant",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(tenant_id, is_legacy_tenant)| Tenant {
        tenant_id,
        is_legacy_tenant,
    })
    .collect())
}

#[instrument(skip(tx), err)]
pub(super) async fn delete_tenant(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: TenantId,
) -> Result<Option<Tenant>, Error> {
    let tenant = QuotedIdentifier::db_name_for_tenant_id(&tenant_id);

    let deleted_tenant = sqlx::query_as::<_, (bool,)>(
        "DELETE FROM management.tenant
           WHERE tenant_id = $1
           RETURNING is_legacy_tenant;",
    )
    .bind(&tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .map(|(is_legacy_tenant,)| Tenant {
        tenant_id,
        is_legacy_tenant,
    });

    if deleted_tenant.is_none() {
        return Ok(None);
    }

    //Hint: $ binds won't work for identifiers (e.g. schema names)
    let query = format!(
        "DROP SCHEMA {tenant} CASCADE;
        DROP ROLE {tenant};"
    );

    tx.execute_many(query.as_str())
        .try_for_each(|_| future::ready(Ok(())))
        .await?;

    Ok(deleted_tenant)
}

#[instrument(skip(tx), err)]
pub(super) async fn create_tenant(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    is_legacy_tenant: bool,
) -> Result<(), Error> {
    let legacy_hint = if is_legacy_tenant {
        LegacyHint::NewSchema
    } else {
        LegacyHint::NotLegacy
    };
    create_tenant_role_and_schema(tx, tenant_id, legacy_hint).await?;
    run_db_migration_for(tx, tenant_id, true).await?;
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum LegacyHint {
    NotLegacy,
    MigrateSchema,
    NewSchema,
}

impl LegacyHint {
    fn is_legacy_tenant(self) -> bool {
        matches!(self, LegacyHint::MigrateSchema | LegacyHint::NewSchema)
    }
}

/// Sets up a new tenant with given id.
///
/// This will fail if the tenant role already exist.
///
/// This will **not** run migrations in the new tenant.
#[instrument(skip(tx), err)]
async fn create_tenant_role_and_schema(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &TenantId,
    legacy_hint: LegacyHint,
) -> Result<(), Error> {
    // TODO make sure legacy tenant creation through management API works
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

    let query = if let LegacyHint::MigrateSchema = legacy_hint {
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

            GRANT ALL
                ON ALL TABLES IN SCHEMA {tenant}
                TO {tenant};

            GRANT ALL
                ON ALL SEQUENCES IN SCHEMA {tenant}
                TO {tenant};

            GRANT ALL
                ON ALL ROUTINES IN SCHEMA {tenant}
                TO {tenant};

            -- make sure all object we create can be used by tenant
            -- Note:
            --   This sets the default privileges for objects created by the user running this
            --   command, this will not affect the privileges of objects created by other users.
            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT ALL
                ON TABLES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT ALL
                ON SEQUENCES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT ALL
                ON ROUTINES
                TO {tenant};

            ALTER DEFAULT PRIVILEGES IN SCHEMA {tenant}
                GRANT ALL
                ON TYPES
                TO {tenant};
        "##
    );

    tx.execute_many(query.as_str())
        .try_for_each(|_| future::ready(Ok(())))
        .await?;

    sqlx::query("INSERT INTO management.tenant (tenant_id, is_legacy_tenant) VALUES ($1, $2);")
        .bind(tenant_id)
        .bind(legacy_hint.is_legacy_tenant())
        .execute(tx)
        .await?;

    Ok(())
}

/// Creates a DB role if it doesn't exist.
#[instrument(skip(pg, followup), err)]
async fn create_role_if_not_exists<F>(
    pg: &Pool<Postgres>,
    role: &QuotedIdentifier,
    followup: impl FnOnce(Transaction<'static, Postgres>) -> F,
) -> Result<bool, Error>
where
    F: Future<Output = Result<(), Error>>,
{
    let mut count = 3;
    loop {
        let mut tx = pg.begin().await?;
        if !does_role_exist(&mut tx, role).await? {
            if let Err(err) = create_role(&mut tx, role).await {
                tx.rollback().await?;
                count -= 1;
                if count > 0 {
                    sleep(Duration::from_millis(100)).await;
                    continue;
                } else {
                    return Err(err);
                }
            }
            followup(tx).await?;
            info!("role created");
            return Ok(true);
        } else {
            tx.commit().await?;
            return Ok(false);
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
            "spurious pg_advisory_unlock which wasn't locked"
        );
    }
    Ok(())
}

/// Generate a `i64` postgres management lock id form a [`TenantId`].
///
/// **There can be collisions**, but less collisions are preferable.
fn generate_tenant_lock_id(tenant_id: &TenantId) -> i64 {
    let mut hasher = DefaultHasher::new();
    tenant_id.hash(&mut hasher);
    let id = hasher.finish() as i64;
    if id == MIGRATION_LOCK_ID {
        // Avoid accidentally colliding with the "general purpose migration
        // lock". This could lead to a dead lock if we try to run per-tenant
        // migrations in their own connection as part of code holding the
        // "general purpose migration lock"
        id + 1
    } else {
        id
    }
}
