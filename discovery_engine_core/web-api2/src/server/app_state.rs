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

use sqlx::{Pool, Postgres};

use crate::embedding::Embedder;

use super::{Config, SetupError};

pub struct AppState<E> {
    #[allow(dead_code)]
    pub(crate) config: Config<E>,
    #[allow(dead_code)]
    pub(crate) db: Pool<Postgres>,
    #[allow(dead_code)]
    pub(crate) embedder: Embedder,
}

impl<E> AppState<E> {
    pub(super) async fn create(config: Config<E>) -> Result<Self, SetupError> {
        let db = config.db.create_connection_pool().await?;
        let embedder = Embedder::load(config.as_ref())?;
        Ok(Self {
            config,
            db,
            embedder,
        })
    }
}
