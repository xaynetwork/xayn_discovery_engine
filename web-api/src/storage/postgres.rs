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

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use async_trait::async_trait;
use itertools::Itertools;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ET-3837")]
use sqlx::types::Json;
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
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{CoiId, CoiStats, NegativeCoi, PositiveCoi, UserInterests};

use super::InteractionUpdateContext;
#[cfg(feature = "ET-3837")]
use crate::models::IngestedDocument;
use crate::{
    error::common::DocumentIdAsObject,
    models::{DocumentId, DocumentTag, InteractedDocument, UserId, UserInteractionType},
    storage::{self, utils::SqlxPushTupleExt, DeletionError, Storage},
    utils::serialize_redacted,
    Error,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Config {
    /// The default base url.
    ///
    /// Passwords in the URL will be ignored, do not set the
    /// db password with the db url.
    base_url: String,

    /// Override port from base url.
    port: Option<u16>,

    /// Override user from base url.
    user: Option<String>,

    /// Sets the password.
    #[serde(serialize_with = "serialize_redacted")]
    password: Secret<String>,

    /// Override db from base url.
    db: Option<String>,

    /// Override default application name from base url.
    application_name: Option<String>,

    /// If true skips running db migrations on start up.
    skip_migrations: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "postgres://user:pw@localhost:5432/xayn".into(),
            port: None,
            user: None,
            password: String::from("pw").into(),
            db: None,
            application_name: option_env!("CARGO_BIN_NAME").map(|name| format!("xayn-web-{name}")),
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

    pub(super) async fn close(&self) {
        self.pool.close().await;
    }

    #[cfg(not(feature = "ET-3837"))]
    pub(super) async fn insert_documents(&self, ids: &[DocumentId]) -> Result<(), Error> {
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

    #[cfg(feature = "ET-3837")]
    pub(super) async fn insert_documents(
        &self,
        documents: impl IntoIterator<Item = &(IngestedDocument, NormalizedEmbedding)>,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        let mut builder = QueryBuilder::new(
            "INSERT INTO document (document_id, snippet, properties, tags, embedding) ",
        );
        let mut documents = documents.into_iter().peekable();
        while documents.peek().is_some() {
            builder
                .reset()
                .push_values(
                    documents.by_ref().take(Self::BIND_LIMIT / 5),
                    |mut builder, (document, embedding)| {
                        builder
                            .push_bind(&document.id)
                            .push_bind(&document.snippet)
                            .push_bind(Json(&document.properties))
                            .push_bind(&document.tags)
                            .push_bind(embedding);
                    },
                )
                .push(
                    " ON CONFLICT (document_id) DO UPDATE SET
                    snippet = EXCLUDED.snippet,
                    properties = EXCLUDED.properties,
                    tags = EXCLUDED.tags,
                    embedding = EXCLUDED.embedding;",
                )
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn delete_documents(&self, ids: &[DocumentId]) -> Result<(), DeletionError> {
        let mut tx = self.pool.begin().await?;

        let documents = self
            .documents_exist_with_transaction(&ids.iter().collect_vec(), &mut tx)
            .await?;
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

        if documents.len() == ids.len() {
            Ok(())
        } else {
            let errors = ids
                .iter()
                .collect::<HashSet<_>>()
                .difference(&documents.iter().collect::<HashSet<_>>())
                .map(|id| DocumentIdAsObject { id: id.to_string() })
                .collect_vec();
            Err(DeletionError::PartialFailure { errors })
        }
    }

    pub(super) async fn document_exists(&self, id: &DocumentId) -> Result<bool, Error> {
        sqlx::query("SELECT document_id FROM document WHERE document_id = $1;")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map(|id| id.is_some())
            .map_err(Into::into)
    }

    pub(super) async fn documents_exist(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<DocumentId>, Error> {
        let mut tx = self.pool.begin().await?;
        let res = self.documents_exist_with_transaction(ids, &mut tx).await?;
        tx.commit().await?;
        Ok(res)
    }

    pub(super) async fn documents_exist_with_transaction(
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
                id: coi.coi_id,
                point: coi.embedding,
                stats: CoiStats {
                    view_count: coi.view_count as usize,
                    view_time: Duration::from_millis(coi.view_time_ms as u64),
                    last_view: coi.last_view,
                },
            })
            .collect_vec();

        let negative = negative
            .into_iter()
            .map(|coi| NegativeCoi {
                id: coi.coi_id,
                point: coi.embedding,
                last_view: coi.last_view,
            })
            .collect_vec();

        Ok(UserInterests { positive, negative })
    }

    /// Update the Center of Interests (COIs).
    ///
    /// This function assumes it will not be called in high amounts
    /// with highly varying numbers of cois. If it is could potentially
    /// lead to degraded global performance of the prepared query
    /// cache. This assumption is unlikely to ever be broken and
    /// even if it's unlikely to actually cause issues.
    async fn upsert_cois(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        time: DateTime<Utc>,
        cois: &HashMap<CoiId, PositiveCoi>,
    ) -> Result<(), Error> {
        let mut builder = QueryBuilder::new(
            "INSERT INTO center_of_interest (
            coi_id, user_id,
            is_positive, embedding,
            view_count, view_time_ms,
            last_view
        ) ",
        );
        let mut iter = cois.values().peekable();
        while iter.peek().is_some() {
            let chunk = iter.by_ref().take(Database::BIND_LIMIT / 7);
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
                        .push_bind(time);
                })
                .push(
                    " ON CONFLICT (coi_id) DO UPDATE SET
                    embedding = EXCLUDED.embedding,
                    view_count = EXCLUDED.view_count,
                    view_time_ms = EXCLUDED.view_time_ms,
                    last_view = EXCLUDED.last_view;",
                )
                .build()
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    async fn upsert_interactions(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        time: DateTime<Utc>,
        interactions: &HashMap<&DocumentId, (&InteractedDocument, UserInteractionType)>,
    ) -> Result<(), Error> {
        //FIXME micro benchmark and chunking+persist abstraction
        let persist = interactions.len() < 10;

        let mut builder = QueryBuilder::new(
            "INSERT INTO interaction (doc_id, user_id, time_stamp, user_reaction)",
        );
        let mut iter = interactions.iter().peekable();
        while iter.peek().is_some() {
            let chunk = iter.by_ref().take(Database::BIND_LIMIT / 4);
            builder
                .reset()
                .push_values(chunk, |mut builder, (document_id, (_, interaction))| {
                    builder
                        .push_bind(document_id)
                        .push_bind(user_id)
                        .push_bind(time)
                        .push_bind(*interaction as i16);
                })
                .push(
                    " ON CONFLICT (doc_id, user_id, time_stamp) DO UPDATE SET
                    user_reaction = EXCLUDED.user_reaction;",
                )
                .build()
                .persistent(persist)
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    async fn upsert_tag_weights(
        tx: &mut Transaction<'_, Postgres>,
        user_id: &UserId,
        updates: &HashMap<&DocumentTag, i32>,
    ) -> Result<(), Error> {
        let mut builder = QueryBuilder::new("INSERT INTO weighted_tag (user_id, tag, weight)");
        let mut iter = updates.iter().peekable();
        while iter.peek().is_some() {
            let chunk = iter.by_ref().take(Database::BIND_LIMIT / 7);
            builder
                .reset()
                .push_values(chunk, |mut builder, (tag, weight_diff)| {
                    builder
                        .push_bind(user_id)
                        .push_bind(tag)
                        .push_bind(weight_diff);
                })
                .push(
                    " ON CONFLICT (user_id, tag) DO UPDATE SET
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
    coi_id: CoiId,
    is_positive: bool,
    embedding: NormalizedEmbedding,
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

#[async_trait(?Send)]
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

    async fn user_seen(&self, id: &UserId, time: DateTime<Utc>) -> Result<(), Error> {
        sqlx::query(
            "INSERT INTO users (user_id, last_seen)
            VALUES ($1, $2)
            ON CONFLICT (user_id)
            DO UPDATE SET last_seen = EXCLUDED.last_seen;",
        )
        .bind(id)
        .bind(time)
        .execute(&self.postgres.pool)
        .await?;

        Ok(())
    }

    async fn update_interactions<F>(
        &self,
        user_id: &UserId,
        updated_document_ids: &[&DocumentId],
        store_user_history: bool,
        time: DateTime<Utc>,
        mut update_logic: F,
    ) -> Result<(), Error>
    where
        F: for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> PositiveCoi + Sync,
    {
        let mut tx = self.postgres.pool.begin().await?;
        Database::acquire_user_coi_lock(&mut tx, user_id).await?;

        let documents = self
            .get_interacted_with_transaction(updated_document_ids, Some(&mut tx))
            .await?;
        let document_map = documents
            .iter()
            .map(|document| (&document.id, (document, UserInteractionType::Positive)))
            .collect::<HashMap<_, _>>();
        let mut tag_weight_diff = documents
            .iter()
            .flat_map(|document| &document.tags)
            .map(|tag| (tag, 0))
            .collect::<HashMap<_, _>>();

        let mut interests = Database::get_user_interests(&mut tx, user_id).await?;
        let mut updates = HashMap::new();
        for id in updated_document_ids {
            if let Some((document, _)) = document_map.get(id) {
                let updated_coi = update_logic(InteractionUpdateContext {
                    document,
                    tag_weight_diff: &mut tag_weight_diff,
                    positive_cois: &mut interests.positive,
                    time,
                });
                // We might update the same coi min `interests` multiple times,
                // if we do we only want to keep the latest update.
                updates.insert(updated_coi.id, updated_coi);
            } else {
                warn!(%id, "interacted document doesn't exist");
            }
        }

        Database::upsert_cois(&mut tx, user_id, time, &updates).await?;
        if store_user_history {
            Database::upsert_interactions(&mut tx, user_id, time, &document_map).await?;
        }
        Database::upsert_tag_weights(&mut tx, user_id, &tag_weight_diff).await?;

        tx.commit().await?;
        Ok(())
    }
}

#[derive(FromRow)]
struct QueriedWeightedTag {
    tag: DocumentTag,
    /// The weight is a `usize` stored as `i32` in database
    weight: i32,
}

#[async_trait]
impl storage::Tag for Storage {
    async fn get(&self, user_id: &UserId) -> Result<HashMap<DocumentTag, usize>, Error> {
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
