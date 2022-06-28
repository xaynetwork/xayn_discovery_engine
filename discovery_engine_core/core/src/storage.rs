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

use std::str::FromStr;

use async_trait::async_trait;
use displaydoc::Display;
use sqlx::{
    sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions},
    Pool,
};
use thiserror::Error;

#[async_trait]
pub trait Storage {
    type StorageError;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError>;
}

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to initialize database: {0}
    Init(#[source] sqlx::Error),
}

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    #[allow(dead_code)]
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub(crate) async fn connect(uri: &str) -> Result<Self, Error> {
        let opt = SqliteConnectOptions::from_str(uri)
            .map_err(Error::Init)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(opt)
            .await
            .map_err(Error::Init)?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    type StorageError = Error;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError> {
        // todo in TY-2971
        Ok(())
    }
}
