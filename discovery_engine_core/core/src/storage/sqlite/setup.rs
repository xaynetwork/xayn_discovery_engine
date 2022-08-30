use std::{path::Path, str::FromStr};

use crate::{
    stack,
    storage::{utils::SqlxPushTupleExt, Error, InitDbHint},
    utils::{remove_file_if_exists, CompoundError, MiscErrorExt},
};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool,
    QueryBuilder,
    Sqlite,
    Transaction,
};

use super::SqliteStorage;

/// Initializes the Sqlite storage system.
///
/// If the opening or schema migrating of an existing db fails it is deleted
/// and a new db is opened instead.
pub(super) async fn init_storage_system(
    file_path: Option<String>,
) -> Result<(SqliteStorage, InitDbHint), Error> {
    let file_exists = db_file_does_exist(file_path.as_deref()).await;
    let result = init_storage_system_once(file_path.clone()).await;
    let first_error = match (file_exists, result) {
        (false, Ok(storage)) => return Ok((storage, InitDbHint::NewDbCreated)),
        (true, Ok(storage)) => return Ok((storage, InitDbHint::NormalInit)),
        (false, Err(err)) => return Err(err),
        (true, Err(err)) => err,
    };

    // either the database was corrupted or we messed up the migrations,
    // as the app must continue working so we start from scratch
    //FIXME we could make a backup so that if we push a bad update the
    //      data could be restored with a follow-up update. But for now
    //      this is fully out of scope of what we have resources for.
    delete_db_files(file_path.as_deref()).await?;

    init_storage_system_once(file_path)
        .await
        .map(|storage| (storage, InitDbHint::DbOverwrittenDueToErrors(first_error)))
}

/// Check if a db file does exist, this must be called before creating a connection pool.
async fn db_file_does_exist(file_path: Option<&str>) -> bool {
    if let Some(path) = file_path {
        //tokio version of `std::path::Path::exists()`
        tokio::fs::metadata(Path::new(path)).await.is_ok()
    } else {
        false
    }
}

/// Deletes all sqlite db files associated with given db file path.
///
/// File does not exist errors are fully ignored.
pub(super) async fn delete_db_files(file_path: Option<&str>) -> Result<(), Error> {
    if let Some(file_path) = &file_path {
        let mut errors = vec![];

        // if deletion fails it is often but not always a problem for the
        // db recreation, so we log the error but do continue on.
        remove_file_if_exists(file_path)
            .await
            .extract_error(&mut errors);
        remove_file_if_exists(&format!("{}-wal", file_path))
            .await
            .extract_error(&mut errors);
        remove_file_if_exists(&format!("{}-shm", file_path))
            .await
            .extract_error(&mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Database(Box::new(CompoundError::new(
                "deleting database files failed",
                errors,
            ))))
        }
    } else {
        Ok(())
    }
}

async fn init_storage_system_once(file_path: Option<String>) -> Result<SqliteStorage, Error> {
    let pool = create_connection_pool(file_path.as_deref()).await?;
    init_database(&pool);
    Ok(SqliteStorage { pool, file_path })
}

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

async fn init_database(pool: &Pool<Sqlite>) -> Result<(), Error> {
    sqlx::migrate!("src/storage/migrations")
        .run(pool)
        .await
        .map_err(|err| Error::Database(err.into()))?;

    let mut tx = pool.begin().await?;
    setup_stacks_sync(&mut tx).await?;
    tx.commit().await?;
    Ok(())
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
