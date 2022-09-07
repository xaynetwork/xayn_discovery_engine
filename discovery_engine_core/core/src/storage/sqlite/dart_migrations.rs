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

//! Module for handling dart->rust/sqltie migrations

use sqlx::{Pool, Sqlite};

use crate::{storage::Error, DartMigrationData};

/// Add the data from the  dart->rust/sqltie migration to the prepared database.
pub(super) async fn store_migration_data(
    pool: &Pool<Sqlite>,
    data: &DartMigrationData,
) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO SerializedState (rowid, state) VALUES (1, ?);")
        .bind(&data.engine_state)
        .execute(&mut tx)
        .await?;

    //TODO[pmk] implement
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::storage::sqlite::setup::{create_connection_pool, update_schema};

    use super::*;

    #[tokio::test]
    async fn test_store_migration_data() {
        let pool = create_connection_pool(None).await.unwrap();
        update_schema(&pool).await.unwrap();

        let expected_state = vec![1, 2, 3, 4, 8, 7, 0];
        store_migration_data(
            &pool,
            &DartMigrationData {
                engine_state: expected_state.clone(),
            },
        )
        .await
        .unwrap();

        let (state,) =
            sqlx::query_as::<_, (Vec<u8>,)>("SELECT state FROM SerializedState WHERE rowid = 1")
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(state, expected_state);
    }
}
