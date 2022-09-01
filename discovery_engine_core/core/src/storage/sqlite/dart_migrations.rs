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
    //TODO[pmk] implement
    Ok(())
}
