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

use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    FromRow,
    Pool,
    Postgres,
};
use xayn_discovery_engine_ai::{CoiSystemState, GenericError};

use crate::models::UserId;

#[derive(Debug, Clone)]
pub(crate) struct UserState {
    pool: Pool<Postgres>,
}

impl UserState {
    pub(crate) async fn connect(uri: &str) -> Result<Self, GenericError> {
        let opt = PgConnectOptions::from_str(uri)?;
        let pool = PgPoolOptions::new().connect_with(opt).await?;
        Ok(Self { pool })
    }

    pub(crate) async fn init_database(&self) -> Result<(), GenericError> {
        sqlx::migrate!("src/migrations").run(&self.pool).await?;
        Ok(())
    }

    pub(crate) async fn fetch(&self, id: &UserId) -> Result<Option<CoiSystemState>, GenericError> {
        let mut tx = self.pool.begin().await?;

        let serialized_state =
            sqlx::query_as::<_, QueriedState>("SELECT state FROM user_state WHERE id = $1;")
                .bind(id.as_ref())
                .fetch_optional(&mut tx)
                .await?;

        tx.commit().await?;

        serialized_state
            .map(|user_state| bincode::deserialize(&user_state.state))
            .transpose()
            .map_err(Into::into)
    }

    pub(crate) async fn update(
        &self,
        id: &UserId,
        state: &CoiSystemState,
    ) -> Result<(), GenericError> {
        let serialized_state = bincode::serialize(state)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO user_state(id, state) VALUES ($1, $2)
                ON CONFLICT (id) DO UPDATE SET state = EXCLUDED.state;",
        )
        .bind(id.as_ref())
        .bind(serialized_state)
        .execute(&mut tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn clear(&self) -> Result<bool, GenericError> {
        let mut tx = self.pool.begin().await?;

        let deletion = sqlx::query("DELETE FROM user_state;")
            .execute(&mut tx)
            .await?;
        tx.commit().await?;

        Ok(deletion.rows_affected() > 0)
    }
}

#[derive(FromRow)]
struct QueriedState {
    state: Vec<u8>,
}
