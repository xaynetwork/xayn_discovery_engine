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

use std::{collections::HashMap, time::Duration};

use itertools::Itertools;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolOptions,
    postgres::PgConnectOptions,
    types::chrono::{DateTime, Utc},
    FromRow,
    Pool,
    Postgres,
    QueryBuilder,
};
use tracing::{info, instrument};
use uuid::Uuid;
use xayn_ai_coi::{CoiStats, Embedding, NegativeCoi, PositiveCoi, UserInterests};

use crate::{
    models::{DocumentId, UserId, UserInteractionType},
    utils::serialize_redacted,
    Error,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// The default base url.
    ///
    /// Passwords in the URL will be ignored, do not set the
    /// db password with the db url.
    #[serde(default = "default_base_url")]
    base_url: String,

    /// Override port from base url.
    #[serde(default)]
    port: Option<u16>,

    /// Override user from base url.
    #[serde(default)]
    user: Option<String>,

    /// Sets the password.
    #[serde(default = "default_password", serialize_with = "serialize_redacted")]
    password: Secret<String>,

    /// Override db from base url.
    #[serde(default)]
    db: Option<String>,

    /// Override default application name from base url.
    ///
    /// Defaults to `xayn-web-{CARGO_BIN_NAME}`.
    #[serde(default = "default_application_name")]
    application_name: Option<String>,

    /// If true skips running db migrations on start up.
    #[serde(default)]
    skip_migrations: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            user: None,
            password: default_password(),
            db: None,
            port: None,
            application_name: default_application_name(),
            skip_migrations: false,
        }
    }
}

fn default_password() -> Secret<String> {
    String::from("pw").into()
}

fn default_base_url() -> String {
    "postgres://user:pw@localhost:5432/xayn".into()
}

fn default_application_name() -> Option<String> {
    option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}"))
}

impl Config {
    #[instrument]
    pub(crate) async fn setup_database(&self) -> Result<Database, sqlx::Error> {
        let options = self.build_connection_options()?;
        info!("starting postgres setup");
        let pool = PoolOptions::new().connect_with(options).await?;
        if !self.skip_migrations {
            sqlx::migrate!().run(&pool).await?;
        }
        Ok(Database { pool })
    }

    fn build_connection_options(&self) -> Result<PgConnectOptions, sqlx::Error> {
        let Self {
            base_url,
            port,
            user,
            password,
            db,
            application_name,
            skip_migrations: _,
        } = &self;

        let mut options = base_url
            .parse::<PgConnectOptions>()?
            .password(password.expose_secret());

        if let Some(user) = user {
            options = options.username(user);
        }
        if let Some(port) = port {
            options = options.port(*port);
        }
        if let Some(db) = db {
            options = options.database(db);
        }
        if let Some(application_name) = application_name {
            options = options.application_name(application_name);
        }

        Ok(options)
    }
}

pub(crate) struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    pub(crate) async fn delete_documents(&self, documents: &[DocumentId]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        QueryBuilder::new("DELETE FROM interaction WHERE doc_id in")
            .push_tuples(documents, |mut query, id| {
                query.push_bind(id);
            })
            .build()
            .persistent(false)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(crate) async fn user_seen(&self, id: &UserId) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO users(user_id, last_seen)
            VALUES ($1, Now())
            ON CONFLICT (user_id)
            DO UPDATE SET last_seen = EXCLUDED.last_seen;",
        )
        .bind(id.as_ref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub(crate) async fn fetch_interests(&self, user_id: &UserId) -> Result<UserInterests, Error> {
        let cois = sqlx::query_as::<_, QueriedCoi>(
            "SELECT coi_id, is_positive, embedding, view_count, view_time_ms, last_view
            FROM center_of_interest
            WHERE user_id = $1",
        )
        .bind(user_id.as_ref())
        .fetch_all(&self.pool)
        .await?;

        let (positive, negative) = cois
            .into_iter()
            .partition::<Vec<_>, _>(|coi| coi.is_positive);

        // fine as we convert it from usize to i32 when we store it in the database
        #[allow(clippy::cast_sign_loss)]
        let positive = positive
            .into_iter()
            .map(|coi| PositiveCoi {
                id: coi.coi_id.into(),
                point: Embedding::from(coi.embedding),
                stats: CoiStats {
                    view_count: coi.view_count as usize,
                    view_time: Duration::from_millis(coi.view_time_ms as u64),
                    last_view: coi.last_view.into(),
                },
            })
            .collect_vec();

        let negative = negative
            .into_iter()
            .map(|coi| NegativeCoi {
                id: coi.coi_id.into(),
                point: Embedding::from(coi.embedding),
                last_view: coi.last_view.into(),
            })
            .collect_vec();

        Ok(UserInterests { positive, negative })
    }

    pub(crate) async fn update_positive_cois<F>(
        &self,
        doc_id: &DocumentId,
        user_id: &UserId,
        update_cois: F,
    ) -> Result<(), Error>
    where
        F: Fn(&mut Vec<PositiveCoi>) -> &PositiveCoi + Send + Sync,
    {
        let mut tx = self.pool.begin().await?;

        sqlx::query("INSERT INTO coi_update_lock (user_id) VALUES ($1) ON CONFLICT DO NOTHING;")
            .bind(user_id)
            .execute(&mut tx)
            .await?;
        sqlx::query("SELECT FROM coi_update_lock WHERE user_id = $1 FOR UPDATE;")
            .bind(user_id)
            .execute(&mut tx)
            .await?;

        // fine as we convert it to i32 when we store it in the database
        #[allow(clippy::cast_sign_loss)]
        let mut positive_cois: Vec<_> = sqlx::query_as::<_, QueriedCoi>(
            "SELECT coi_id, is_positive, embedding, view_count, view_time_ms, last_view
            FROM center_of_interest
            WHERE user_id = $1 AND is_positive;",
        )
        .bind(user_id)
        .fetch_all(&mut tx)
        .await?
        .into_iter()
        .map(|coi| PositiveCoi {
            id: coi.coi_id.into(),
            point: Embedding::from(coi.embedding),
            stats: CoiStats {
                view_count: coi.view_count as usize,
                view_time: Duration::from_millis(coi.view_time_ms as u64),
                last_view: coi.last_view.into(),
            },
        })
        .collect();

        let updated_coi = update_cois(&mut positive_cois);
        let timestamp: DateTime<Utc> = updated_coi.stats.last_view.into();

        // bit casting to signed int is fine as we fetch them as signed int before bit casting them back to unsigned int
        // truncating to 64bit is fine as >292e+6 years is more then enough for this use-case
        #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
        sqlx::query(
            "INSERT INTO center_of_interest (coi_id, user_id, is_positive, embedding, view_count, view_time_ms, last_view)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (coi_id) DO UPDATE SET
                embedding = EXCLUDED.embedding,
                view_count = EXCLUDED.view_count,
                view_time_ms = EXCLUDED.view_time_ms,
                last_view = EXCLUDED.last_view;",
        )
        .bind(updated_coi.id.as_ref())
        .bind(user_id)
        .bind(true)
        .bind(updated_coi.point.to_vec())
        .bind(updated_coi.stats.view_count as i32)
        .bind(updated_coi.stats.view_time.as_millis() as i64)
        .bind(timestamp)
        .execute(&mut tx)
        .await?;

        sqlx::query(
            "INSERT INTO interaction (doc_id, user_id, time_stamp, user_reaction)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (doc_id, user_id, time_stamp) DO UPDATE SET
                user_reaction = EXCLUDED.user_reaction;",
        )
        .bind(doc_id)
        .bind(user_id)
        .bind(timestamp)
        .bind(UserInteractionType::Positive as i16)
        .execute(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn fetch_interacted_document_ids(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DocumentId>, Error> {
        let mut tx = self.pool.begin().await?;

        let documents = sqlx::query_as::<_, QueriedInteractedDocumentId>(
            "SELECT DISTINCT doc_id
            FROM interaction
            WHERE user_id = $1;",
        )
        .bind(user_id.as_ref())
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(documents.into_iter().map_into().collect())
    }

    pub(crate) async fn fetch_category_weights(
        &self,
        user_id: &UserId,
    ) -> Result<HashMap<String, usize>, Error> {
        let mut tx = self.pool.begin().await?;

        let categories = sqlx::query_as::<_, QueriedWeightedCategory>(
            "SELECT (category, weight)
            FROM weighted_category
            WHERE user_id = $1;",
        )
        .bind(user_id)
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(categories
            .into_iter()
            .map(|category| (category.category, category.weight as usize))
            .collect())
    }

    pub(crate) async fn update_category_weight(
        &self,
        user_id: &UserId,
        category: &str,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO weighted_category (user_id, category, weight)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, category) DO UPDATE SET
                weight = weighted_category.weight + 1;",
        )
        .bind(user_id.as_ref())
        .bind(category)
        .bind(1)
        .execute(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

#[derive(FromRow)]
struct QueriedCoi {
    coi_id: Uuid,
    is_positive: bool,
    embedding: Vec<f32>,
    /// The count is a `usize` stored as `i32` in database
    view_count: i32,
    /// The time is a `u64` stored as `i64` in database
    view_time_ms: i64,
    last_view: DateTime<Utc>,
}

#[derive(FromRow)]
struct QueriedInteractedDocumentId {
    //FIXME this should be called `document_id`
    doc_id: DocumentId,
}

impl From<QueriedInteractedDocumentId> for DocumentId {
    fn from(document_id: QueriedInteractedDocumentId) -> Self {
        document_id.doc_id
    }
}

#[derive(FromRow)]
struct QueriedWeightedCategory {
    category: String,
    /// The weight is a `usize` stored as `i32` in database
    weight: i32,
}
