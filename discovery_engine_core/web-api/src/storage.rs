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

use std::{str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use ndarray::Array;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    FromRow,
    Pool,
    Postgres,
    QueryBuilder,
};
use uuid::Uuid;
use xayn_discovery_engine_ai::{
    CoiStats,
    Embedding,
    GenericError,
    NegativeCoi,
    PositiveCoi,
    UserInterests,
};

use crate::models::UserId;

// PostgreSQL bind limit
const BIND_LIMIT: usize = 65535;

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

    pub(crate) async fn fetch(&self, id: &UserId) -> Result<Option<UserInterests>, GenericError> {
        let mut tx = self.pool.begin().await?;

        let cois = sqlx::query_as::<_, QueriedCoi>(
            "SELECT coi_id, is_positive, embedding, view_count, view_time_ms, last_view 
            FROM center_of_interest 
            WHERE user_id = $1",
        )
        .bind(id.as_ref())
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        let (positive, negative): (Vec<_>, Vec<_>) =
            cois.into_iter().partition(|coi| coi.is_positive);

        // fine as we convert it to i32 when we store it in the database
        #[allow(clippy::cast_sign_loss)]
        let positive: Vec<_> = positive
            .into_iter()
            .map(|coi| PositiveCoi {
                id: coi.coi_id.into(),
                point: Embedding::from(Array::from_vec(coi.embedding)),
                stats: CoiStats {
                    view_count: coi.view_count as usize,
                    view_time: Duration::from_millis(coi.view_time_ms as u64),
                    last_view: coi.last_view.into(),
                },
            })
            .collect();

        let negative: Vec<_> = negative
            .into_iter()
            .map(|coi| NegativeCoi {
                id: coi.coi_id.into(),
                point: Embedding::from(Array::from_vec(coi.embedding)),
                last_view: coi.last_view.into(),
            })
            .collect();

        Ok(Some(UserInterests { positive, negative }))
    }

    pub(crate) async fn update(
        &self,
        id: &UserId,
        user_interests: &UserInterests,
    ) -> Result<(), GenericError> {
        let positive_cois = &user_interests.positive;
        let negative_cois = &user_interests.negative;

        let mut tx = self.pool.begin().await?;
        let mut query_builder = QueryBuilder::new("INSERT INTO ");

        // The amount of rows that we can store via bulk inserts
        // (<https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values>)
        // is limited by the postgreSQL bind limit. Hence, the BIND_LIMIT is divided by the number of
        // fields in the largest tuple.
        for cois in positive_cois.chunks(BIND_LIMIT / 7) {
            query_builder
                .reset()
                .push("center_of_interest (coi_id, user_id, is_positive, embedding, view_count, view_time_ms, last_view) ")
                .push_values(cois, |mut stm, coi| {
                    let timestamp: DateTime<Utc> = coi.stats.last_view.into();

                    // fine as we convert it back to usize/u64 when we fetch it from the database
                    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
                    stm.push_bind(coi.id.as_ref())
                    .push_bind(id.as_ref())
                    .push_bind(true)
                    .push_bind(coi.point.to_vec())
                        .push_bind(coi.stats.view_count as i32)
                        .push_bind(coi.stats.view_time.as_millis() as i32)
                        .push_bind(timestamp);
                })
                .push(
                    " ON CONFLICT (coi_id) DO UPDATE SET
                        embedding = EXCLUDED.embedding,
                        view_count = EXCLUDED.view_count,
                        view_time_ms = EXCLUDED.view_time_ms,
                        last_view = EXCLUDED.last_view;"
                )
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;
        }

        for cois in negative_cois.chunks(BIND_LIMIT / 5) {
            query_builder
                .reset()
                .push("center_of_interest (coi_id, user_id, is_positive, embedding, last_view) ")
                .push_values(cois, |mut stm, coi| {
                    let timestamp: DateTime<Utc> = coi.last_view.into();

                    stm.push_bind(coi.id.as_ref())
                        .push_bind(id.as_ref())
                        .push_bind(false)
                        .push_bind(coi.point.to_vec())
                        .push_bind(timestamp);
                })
                .push(
                    " ON CONFLICT (coi_id) DO UPDATE SET
                        embedding = EXCLUDED.embedding,
                        last_view = EXCLUDED.last_view;",
                )
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn clear(&self) -> Result<bool, GenericError> {
        let mut tx = self.pool.begin().await?;

        let deletion = sqlx::query("DELETE FROM center_of_interest;")
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(deletion.rows_affected() > 0)
    }
}

#[derive(FromRow)]
struct QueriedCoi {
    coi_id: Uuid,
    is_positive: bool,
    embedding: Vec<f32>,
    view_count: i32,
    view_time_ms: i32,
    last_view: DateTime<Utc>,
}
