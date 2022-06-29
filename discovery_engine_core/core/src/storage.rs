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

use async_trait::async_trait;

use crate::document::HistoricDocument;

pub mod sqlite;

#[async_trait]
pub trait Storage {
    type StorageError;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError>;

    async fn fetch_history(&self)
        -> Result<Vec<HistoricDocument>, <Self as Storage>::StorageError>;
}
