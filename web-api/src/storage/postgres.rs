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

use async_trait::async_trait;
use itertools::Itertools;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{
    pool::PoolOptions,
    postgres::PgConnectOptions,
    types::chrono::{DateTime, Utc},
    Executor,
    FromRow,
    Pool,
    Postgres,
    QueryBuilder,
    Transaction,
};
use tracing::{info, instrument, warn};
use uuid::Uuid;
use xayn_ai_coi::{CoiStats, Embedding, NegativeCoi, PositiveCoi, UserInterests};

use super::InteractionUpdateContext;
use crate::{
    models::{DocumentId, UserId, UserInteractionType},
    storage::{self, utils::SqlxPushTupleExt, Storage},
    utils::serialize_redacted,
    Error,
};

fn default_base_url() -> String {
    "postgres://user:pw@localhost:5432/xayn".into()
}

fn default_password() -> Secret<String> {
    String::from("pw").into()
}

fn default_application_name() -> Option<String> {
    option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}"))
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Config {
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

impl Config {
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
}

pub(crate) struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    // https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html#note-database-specific-limits
    const BIND_LIMIT: usize = 65_535;

    pub(crate) async fn insert_documents(&self, ids: &[DocumentId]) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        let mut builder = QueryBuilder::new("INSERT INTO document (document_id) ");
        for ids in ids.chunks(Self::BIND_LIMIT) {
            builder
                .reset()
                .push_values(ids, |mut builder, id| {
                    builder.push_bind(id);
                })
                .push(" ON CONFLICT DO NOTHING;")
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn delete_documents(&self, ids: &[DocumentId]) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        let mut builder = QueryBuilder::new("DELETE FROM document WHERE document_id IN ");
        for ids in ids.chunks(Self::BIND_LIMIT) {
            builder
                .reset()
                .push_tuple(ids)
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn document_exists(&self, id: &DocumentId) -> Result<bool, Error> {
        sqlx::query("SELECT document_id FROM document WHERE document_id = $1;")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map(|id| id.is_some())
            .map_err(Into::into)
    }

    pub(crate) async fn documents_exist(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<DocumentId>, Error> {
        let mut tx = self.pool.begin().await?;
        let res = self.documents_exist_with_transaction(ids, &mut tx).await?;
        tx.commit().await?;
        Ok(res)
    }

    pub(crate) async fn documents_exist_with_transaction(
        &self,
        ids: &[&DocumentId],
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Vec<DocumentId>, Error> {
        let mut builder =
            QueryBuilder::new("SELECT document_id FROM document WHERE document_id IN ");
        let mut existing_ids = Vec::with_capacity(ids.len());
        for ids in ids.chunks(Self::BIND_LIMIT) {
            for id in builder
                .reset()
                .push_tuple(ids)
                .build()
                .fetch_all(&mut *tx)
                .await?
            {
                existing_ids.push(DocumentId::from_row(&id)?);
            }
        }
        Ok(existing_ids)
    }

    async fn acquire_user_coi_lock(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
    ) -> Result<(), Error> {
        // locks db for given user for coi update context
        sqlx::query("INSERT INTO coi_update_lock (user_id) VALUES ($1) ON CONFLICT DO NOTHING;")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("SELECT FROM coi_update_lock WHERE user_id = $1 FOR UPDATE;")
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    async fn get_user_interests(
        tx: impl Executor<'_, Database = Postgres>,
        user_id: &UserId,
    ) -> Result<UserInterests, Error> {
        let cois = sqlx::query_as::<_, QueriedCoi>(
            "SELECT coi_id, is_positive, embedding, view_count, view_time_ms, last_view
            FROM center_of_interest
            WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(tx)
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

    async fn upsert_cois(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        now: DateTime<Utc>,
        cois: &[PositiveCoi],
    ) -> Result<(), Error> {
        let persist = match cois.len() {
            0 => return Ok(()),
            1 => true,
            _ => false,
        };

        let mut builder = QueryBuilder::new(
            "INSERT INTO center_of_interest (
            coi_id, user_id,
            is_positive, embedding,
            view_count, view_time_ms,
            last_view
        ) ",
        );
        for chunk in cois.chunks(Database::BIND_LIMIT / 7) {
            builder
                .reset()
                .push_values(chunk, |mut builder, update| {
                    // bit casting to signed int is fine as we fetch them as signed int before bit casting them back to unsigned int
                    // truncating to 64bit is fine as >292e+6 years is more then enough for this use-case
                    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
                    builder
                        .push_bind(update.id.as_ref())
                        .push_bind(user_id)
                        .push_bind(true)
                        .push_bind(update.point.to_vec())
                        .push_bind(update.stats.view_count as i32)
                        .push_bind(update.stats.view_time.as_millis() as i64)
                        .push_bind(now);
                })
                .push(
                    " ON CONFLICT (coi_id) DO UPDATE SET
                    embedding = EXCLUDED.embedding,
                    view_count = EXCLUDED.view_count,
                    view_time_ms = EXCLUDED.view_time_ms,
                    last_view = EXCLUDED.last_view;",
                )
                .build()
                .persistent(persist)
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    async fn upsert_interactions<'d, I>(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        now: DateTime<Utc>,
        interactions: I,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = (&'d DocumentId, UserInteractionType)>,
        I::IntoIter: ExactSizeIterator,
    {
        let interactions = interactions.into_iter();
        let persist = match interactions.len() {
            0 => return Ok(()),
            1 => true,
            _ => false,
        };

        let mut builder = QueryBuilder::new(
            "INSERT INTO interaction (doc_id, user_id, time_stamp, user_reaction)",
        );
        let mut iter = interactions.peekable();
        while iter.peek().is_some() {
            let chunk = (&mut iter).take(Database::BIND_LIMIT / 4);
            builder
                .reset()
                .push_values(chunk, |mut builder, (document_id, interaction)| {
                    builder
                        .push_bind(document_id)
                        .push_bind(user_id)
                        .push_bind(now)
                        .push_bind(interaction as i16);
                })
                .push(
                    "ON CONFLICT (doc_id, user_id, time_stamp) DO UPDATE SET
                    user_reaction = EXCLUDED.user_reaction;",
                )
                .build()
                .persistent(persist)
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    async fn upsert_tag_weights<'c, I>(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        updates: I,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = (&'c str, i32)>,
    {
        let updates = updates.into_iter();

        let mut builder = QueryBuilder::new("INSERT INTO weighted_tag (user_id, tag, weight)");
        let mut iter = updates.into_iter().peekable();
        while iter.peek().is_some() {
            let chunk = (&mut iter).take(Database::BIND_LIMIT / 7);
            builder
                .reset()
                .push_values(chunk, |mut builder, (tag, weight_diff)| {
                    builder
                        .push_bind(user_id)
                        .push_bind(tag)
                        .push_bind(weight_diff);
                })
                .push(
                    "ON CONFLICT (user_id, tag) DO UPDATE SET
                    weight = weighted_tag.weight + EXCLUDED.weight;",
                )
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;
        }
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

#[async_trait]
impl storage::Interest for Storage {
    async fn get(&self, user_id: &UserId) -> Result<UserInterests, Error> {
        Database::get_user_interests(&self.postgres.pool, user_id).await
    }
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

#[async_trait]
impl storage::Interaction for Storage {
    async fn get(&self, user_id: &UserId) -> Result<Vec<DocumentId>, Error> {
        let mut tx = self.postgres.pool.begin().await?;

        let documents = sqlx::query_as::<_, QueriedInteractedDocumentId>(
            "SELECT DISTINCT doc_id
            FROM interaction
            WHERE user_id = $1;",
        )
        .bind(user_id)
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(documents.into_iter().map_into().collect())
    }

    async fn user_seen(&self, id: &UserId) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO users (user_id, last_seen)
            VALUES ($1, Now())
            ON CONFLICT (user_id)
            DO UPDATE SET last_seen = EXCLUDED.last_seen;",
        )
        .bind(id)
        .execute(&self.postgres.pool)
        .await?;

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn update_interactions<F>(
        &self,
        user_id: &UserId,
        updated_document_ids: &[&DocumentId],
        mut update_logic: F,
    ) -> Result<(), Error>
    where
        F: for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> PositiveCoi + Send + Sync,
    {
        let mut tx = self.postgres.pool.begin().await?;

        Database::acquire_user_coi_lock(&mut tx, user_id).await?;

        let documents = self
            .get_by_ids_with_transaction(updated_document_ids, Some(&mut tx))
            .await?;

        let now = Utc::now();

        let mut document_map = documents
            .iter()
            .map(|d| (&d.id, d))
            .collect::<HashMap<_, _>>();

        let mut interests = Database::get_user_interests(&mut tx, user_id).await?;

        let mut tag_weight_diff = documents
            .iter()
            .flat_map(|d| &d.tags)
            .map(|tag| (tag.as_str(), 0))
            .collect::<HashMap<_, _>>();

        let mut updates = Vec::new();

        for id in updated_document_ids {
            if let Some(document) = document_map.get_mut(id) {
                updates.push(update_logic(InteractionUpdateContext {
                    document,
                    tag_weight_diff: &mut tag_weight_diff,
                    positive_cois: &mut interests.positive,
                }));
            } else {
                warn!(%id, "interacted document doesn't exist");
            }
        }

        Database::upsert_cois(&mut tx, user_id, now, &updates).await?;

        Database::upsert_interactions(
            &mut tx,
            user_id,
            now,
            document_map
                .values()
                .map(|d| (&d.id, UserInteractionType::Positive))
                // without the collect rust fails with `error: higher-ranked lifetime error`
                // this seems to be a limitation or bug in rustc
                .collect_vec(),
        )
        .await?;

        Database::upsert_tag_weights(&mut tx, user_id, tag_weight_diff.into_iter()).await?;

        tx.commit().await?;
        Ok(())
    }
}

#[derive(FromRow)]
struct QueriedWeightedTag {
    tag: String,
    /// The weight is a `usize` stored as `i32` in database
    weight: i32,
}

#[async_trait]
impl storage::Tag for Storage {
    async fn get(&self, user_id: &UserId) -> Result<HashMap<String, usize>, Error> {
        let mut tx = self.postgres.pool.begin().await?;

        let tags = sqlx::query_as::<_, QueriedWeightedTag>(
            "SELECT tag, weight
            FROM weighted_tag
            WHERE user_id = $1;",
        )
        .bind(user_id)
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(tags
            .into_iter()
            .map(
                #[allow(clippy::cast_sign_loss)] // the weight originally was a usize
                |tag| (tag.tag, tag.weight as usize),
            )
            .collect())
    }
}
