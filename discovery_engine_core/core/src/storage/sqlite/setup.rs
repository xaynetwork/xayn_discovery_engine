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

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    str::FromStr,
};

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
};

use super::SqliteStorage;

/// Initializes the Sqlite storage system.
///
/// If the opening or schema migrating of an existing db fails it is deleted
/// and a new db is opened instead.
pub(super) async fn init_storage_system(
    file_path: Option<PathBuf>,
) -> Result<(SqliteStorage, InitDbHint), Error> {
    let file_exists = if let Some(path) = &file_path {
        //tokio version of `std::path::Path::exists()`
        tokio::fs::metadata(path).await.is_ok()
    } else {
        false
    };
    let result = init_storage_system_once(file_path.clone()).await;
    match (file_exists, result) {
        (false, Ok(storage)) => Ok((storage, InitDbHint::NewDbCreated)),
        (true, Ok(storage)) => Ok((storage, InitDbHint::NormalInit)),
        (false, Err(err)) => Err(err),
        (true, Err(first_error)) => {
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
    }
}

/// Deletes all sqlite db files associated with given db file path.
///
/// File does not exist errors are fully ignored.
pub(super) async fn delete_db_files(file_path: Option<&Path>) -> Result<(), Error> {
    if let Some(file_path) = &file_path {
        let mut errors = vec![];

        // if deletion fails it is often but not always a problem for the
        // db recreation, so we log the error but do continue on.
        remove_file_if_exists(file_path)
            .await
            .extract_error(&mut errors);
        remove_file_if_exists(with_file_name_suffix(file_path, "-wal")?)
            .await
            .extract_error(&mut errors);
        remove_file_if_exists(with_file_name_suffix(file_path, "-shm")?)
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

fn with_file_name_suffix(path: &Path, suffix: &str) -> Result<PathBuf, Error> {
    let mut file_name = path
        .file_name()
        .ok_or_else(|| {
            Error::Database(format!("path doesn't have a file name: {}", path.display()).into())
        })?
        .to_owned();
    file_name.push(OsStr::new(suffix));
    Ok(path.with_file_name(file_name))
}

async fn init_storage_system_once(file_path: Option<PathBuf>) -> Result<SqliteStorage, Error> {
    let pool = create_connection_pool(file_path.as_deref()).await?;
    update_schema(&pool).await?;
    update_static_data(&pool).await?;
    Ok(SqliteStorage { pool })
}

pub(super) async fn create_connection_pool(
    file_path: Option<&Path>,
) -> Result<Pool<Sqlite>, Error> {
    let opt = if let Some(file_path) = &file_path {
        SqliteConnectOptions::new().filename(file_path)
    } else {
        SqliteConnectOptions::from_str("sqlite::memory:")?
    }
    .create_if_missing(true);

    SqlitePoolOptions::new()
        .connect_with(opt)
        .await
        .map_err(Into::into)
}

async fn update_schema(pool: &Pool<Sqlite>) -> Result<(), Error> {
    sqlx::migrate!("src/storage/migrations")
        .run(pool)
        .await
        .map_err(|err| Error::Database(err.into()))
}

//FIXME at some point we probably should derive this from the configuration
//      and also decide what to do with documents in the history which are
//      associated with a stack which is not included in the current config
const EXISTING_STACKS: [stack::Id; 4] = [
    stack::ops::breaking::BreakingNews::id(),
    stack::ops::personalized::PersonalizedNews::id(),
    stack::ops::trusted::TrustedNews::id(),
    stack::exploration::Stack::id(),
];

async fn update_static_data(pool: &Pool<Sqlite>) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    let mut query_builder = QueryBuilder::new(String::new());
    query_builder
        .push("INSERT INTO Stack (stackId) ")
        .push_values(EXISTING_STACKS, |mut stm, id| {
            stm.push_bind(id);
        })
        .push(" ON CONFLICT DO NOTHING;")
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;
    query_builder
        .reset()
        .push("DELETE FROM Stack WHERE stackId NOT IN ")
        .push_tuple(EXISTING_STACKS)
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tempfile::tempdir;

    use crate::storage::Storage;

    use super::*;

    #[tokio::test]
    async fn test_missing_stacks_are_added_and_removed_stacks_removed() {
        let pool = create_connection_pool(None).await.unwrap();

        update_schema(&pool).await.unwrap();

        let mut tx = pool.begin().await.unwrap();
        let random_id = stack::Id::new_random();
        sqlx::query("INSERT INTO Stack(stackId) VALUES (?), (?);")
            .bind(stack::PersonalizedNews::id())
            .bind(random_id)
            .execute(&mut tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        update_static_data(&pool).await.unwrap();

        let ids = sqlx::query_as::<_, stack::Id>("SELECT stackId FROM Stack;")
            .fetch_all(&pool)
            .await
            .unwrap();

        assert_eq!(
            ids.into_iter().collect::<HashSet<_>>(),
            EXISTING_STACKS.into_iter().collect::<HashSet<_>>()
        );
    }

    fn create_bad_file(path: &Path) {
        std::fs::write(path, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 77, 88, 112]).unwrap();
    }

    #[tokio::test]
    async fn test_db_deletion_deletes_all_db_related_files() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join("db.sqlite");
        let db_wal_file = dir.path().join("db.sqlite-wal");
        let db_shm_file = dir.path().join("db.sqlite-shm");

        create_bad_file(&db_file);
        create_bad_file(&db_wal_file);
        create_bad_file(&db_shm_file);

        assert!(db_file.exists());
        assert!(db_wal_file.exists());
        assert!(db_shm_file.exists());

        delete_db_files(Some(&db_file)).await.unwrap();

        assert!(!db_file.exists());
        assert!(!db_wal_file.exists());
        assert!(!db_shm_file.exists());
    }

    #[tokio::test]
    async fn test_db_deletion_ignores_non_existing_files() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join("db.sqlite");
        let db_wal_file = dir.path().join("db.sqlite-wal");
        let db_shm_file = dir.path().join("db.sqlite-shm");

        create_bad_file(&db_shm_file);

        assert!(!db_file.exists());
        assert!(!db_wal_file.exists());
        assert!(db_shm_file.exists());

        delete_db_files(Some(&db_file)).await.unwrap();

        assert!(!db_file.exists());
        assert!(!db_wal_file.exists());
        assert!(!db_shm_file.exists());
    }

    #[tokio::test]
    async fn test_db_deletion_does_not_short_circuit_on_error() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join("db.sqlite");
        let db_wal_file = dir.path().join("db.sqlite-wal");
        let db_shm_file = dir.path().join("db.sqlite-shm");

        create_bad_file(&db_file);
        std::fs::create_dir(&db_wal_file).unwrap();
        create_bad_file(&db_shm_file);

        assert!(db_file.exists());
        assert!(db_wal_file.exists());
        assert!(db_shm_file.exists());

        let res = delete_db_files(Some(&db_file)).await;
        assert!(res.is_err());

        assert!(!db_file.exists());
        assert!(db_wal_file.exists());
        assert!(!db_shm_file.exists());
    }

    #[tokio::test]
    async fn test_db_deletion_can_be_called_without_path() {
        delete_db_files(None).await.unwrap();
    }

    #[tokio::test]
    async fn test_initialization_will_retry_on_bad_db() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join("db.sqlite");

        create_bad_file(&db_file);
        assert!(db_file.exists());

        let (storage, hint) = init_storage_system(Some(db_file.clone())).await.unwrap();

        assert!(db_file.exists());

        assert!(matches!(hint, InitDbHint::DbOverwrittenDueToErrors(_)));

        // check if we can interact with the db
        storage.fetch_history().await.unwrap();
    }

    #[tokio::test]
    async fn test_fresh_db_and_reopened_db_return_the_right_hints() {
        let dir = tempdir().unwrap();
        let db_file = dir.path().join("db.sqlite");

        let (storage, hint) = init_storage_system(Some(db_file.clone())).await.unwrap();

        assert!(db_file.exists());
        assert!(matches!(hint, InitDbHint::NewDbCreated));
        // check if we can interact with the db
        storage.fetch_history().await.unwrap();
        drop(storage);

        assert!(db_file.exists());

        let (storage, hint) = init_storage_system(Some(db_file.clone())).await.unwrap();

        assert!(db_file.exists());
        assert!(matches!(hint, InitDbHint::NormalInit));
        // check if we can interact with the db
        storage.fetch_history().await.unwrap();
        drop(storage);
    }
}
