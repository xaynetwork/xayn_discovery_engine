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

use itertools::Itertools;

use crate::{
    storage::{Error, Storage},
    storage2::DartMigrationData,
};

use super::SqliteStorage;

/// Add the data from the  dart->rust/sqltie migration to the prepared database.
pub(super) async fn store_migration_data(
    storage: &mut SqliteStorage,
    data: &DartMigrationData,
) -> Result<(), Error> {
    // it's okay to not have an transaction across the various migrations:
    // 1. by taking `&mut SqliteStorage` we know we have exclusive access
    // 2. databases of failed migrations should be discarded at some point
    // 3. even if the database is not discarded the db is still in a sound state,
    //    just with some history/config/preference or similar missing

    if let Some(engine_state) = &data.engine_state {
        storage.state().store(engine_state).await?;
    }

    storage
        .source_preference()
        .set_trusted(&data.trusted_sources.iter().map_into().collect())
        .await?;

    storage
        .source_preference()
        .set_excluded(&data.excluded_sources.iter().map_into().collect())
        .await?;

    //FIXME handle documents
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{super::setup::init_storage_system_once, *};

    #[tokio::test]
    async fn test_store_migration_data() {
        let data = DartMigrationData {
            engine_state: Some(vec![1, 2, 3, 4, 8, 7, 0]),
            trusted_sources: vec!["foo.example".into(), "bar.invalid".into()],
            excluded_sources: vec!["dodo.local".into()],
            documents: vec![],
        };
        let storage = init_storage_system_once(None, Some(&data)).await.unwrap();
        let engine_state = storage.state().fetch().await.unwrap();
        let trusted_sources = storage.source_preference().fetch_trusted().await.unwrap();
        let excluded_sources = storage.source_preference().fetch_excluded().await.unwrap();

        assert_eq!(engine_state, data.engine_state);
        assert_eq!(trusted_sources, data.trusted_sources.into_iter().collect());
        assert_eq!(
            excluded_sources,
            data.excluded_sources.into_iter().collect()
        );

        //FIXME test documents search, feed, with history and without history
    }
}
