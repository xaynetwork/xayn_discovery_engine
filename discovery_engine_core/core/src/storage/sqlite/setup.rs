use std::str::FromStr;

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool,
    QueryBuilder,
    Sqlite,
    SqlitePool,
    Transaction,
};
use tokio::fs;

use crate::{
    stack,
    storage::{utils::SqlxPushTupleExt, Error, InitDbHint},
};

use super::SqliteStorage;

async fn create_connection_pool(file_path: Option<&str>) -> Result<Pool<Sqlite>, Error> {
    let opt = if let Some(file_path) = &file_path {
        SqliteConnectOptions::from_str(&format!("sqlite:{}", file_path))
    } else {
        SqliteConnectOptions::from_str("sqlite::memory:")
    }?
    .create_if_missing(true);

    SqlitePoolOptions::new()
        .connect_with(opt)
        .await
        .map_err(Into::into)
}

pub async fn init_storage_system(
    file_path: Option<String>,
) -> Result<(SqliteStorage, InitDbHint), Error> {
    let first_error = match init_storage_system_once(file_path.clone()).await {
        (true, Ok(storage)) => return Ok((storage, InitDbHint::NewDbCreated)),
        (false, Ok(storage)) => return Ok((storage, InitDbHint::NormalInit)),
        (true, Err(err)) => return Err(err),
        (false, Err(err)) => err,
    };

    if let Some(file_path) = &file_path {
        fs::remove_file(file_path).await;
        fs::remove_file(&format!("{}-wal", file_path)).await;
        fs::remove_file(&format!("{}-shm", file_path)).await;
    }

    init_storage_system_once(file_path)
        .await
        .map(|storage| (storage, InitDbHint::DbOverwrittenDueToErrors(first_error)))
}

pub async fn init_storage_system_once(
    file_path: Option<String>,
) -> Result<(bool, SqliteStorage), Error> {
    let pool = create_connection_pool(file_path.as_deref()).await?;
    let fresh = query_if_db_is_empty(&pool).await?;
    //todo[pmk]
}

pub(super) async fn init_database(pool: &Pool<Sqlite>) -> Result<(), Error> {
    sqlx::migrate!("src/storage/migrations")
        .run(pool)
        .await
        .map_err(|err| Error::Database(err.into()))?;

    let mut tx = pool.begin().await?;
    setup_stacks_sync(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Returns true if there are no tables in the db.
async fn query_if_db_is_empty(pool: &Pool<Sqlite>) -> Result<bool, Error> {
    let (count,) = sqlx::query_as::<_, (u32,)>("SELECT count(*) FROM sqlite_schema")
        .fetch_one(pool)
        .await?;

    Ok(count == 0)
}

async fn setup_stacks_sync(tx: &mut Transaction<'_, Sqlite>) -> Result<(), Error> {
    let expected_ids = &[
        stack::ops::breaking::BreakingNews::id(),
        stack::ops::personalized::PersonalizedNews::id(),
        stack::ops::trusted::TrustedNews::id(),
        stack::exploration::Stack::id(),
    ];

    let mut query_builder = QueryBuilder::new(String::new());
    query_builder
        .push("INSERT INTO Stack (stackId) ")
        .push_values(expected_ids, |mut stm, id| {
            stm.push_bind(id);
        })
        .push(" ON CONFLICT DO NOTHING;")
        .build()
        .persistent(false)
        .execute(&mut *tx)
        .await?;

    query_builder
        .reset()
        .push("DELETE FROM Stack WHERE stackId NOT IN ")
        .push_tuple(expected_ids)
        .build()
        .persistent(false)
        .execute(tx)
        .await?;
    Ok(())
}
