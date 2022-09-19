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

use crate::{
    storage::{Error, Storage},
    DartMigrationData,
};

use super::SqliteStorage;

/// Add the data from the  dart->rust/sqltie migration to the prepared database.
pub(super) async fn store_migration_data(
    storage: &mut SqliteStorage,
    data: &DartMigrationData,
) -> Result<(), Error> {
    // it's okay to not have a transaction across the various migrations:
    // 1. by taking `&mut SqliteStorage` we know we have exclusive access
    // 2. databases of failed migrations should be discarded at some point
    // 3. even if the database is not discarded the db is still in a valid state,
    //    just with some history/config/preference or similar missing

    if let Some(engine_state) = &data.engine_state {
        storage.state().store(engine_state).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{super::setup::init_storage_system_once, *};

    #[tokio::test]
    async fn test_store_migration_data() {
        let data = DartMigrationData {
            engine_state: Some(vec![1, 2, 3, 4, 8, 7, 0]),
        };
        let storage = init_storage_system_once(None, Some(&data)).await.unwrap();
        let engine_state = storage.state().fetch().await.unwrap();
        assert_eq!(engine_state, data.engine_state);
    }
}
