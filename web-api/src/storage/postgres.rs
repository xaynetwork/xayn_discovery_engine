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

mod client;

use std::{
    collections::{HashMap, HashSet},
    slice,
    time::Duration,
};

use async_trait::async_trait;
pub(crate) use client::{Database, DatabaseBuilder};
use either::Either;
use futures_util::{future, TryStreamExt};
use itertools::Itertools;
use serde_json::Value;
use sqlx::{
    postgres::PgRow,
    types::{
        chrono::{DateTime, Utc},
        Json,
    },
    Executor,
    FromRow,
    Postgres,
    QueryBuilder,
    Row,
    Transaction,
};
use tracing::{info, instrument};
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{Coi, CoiId, CoiStats};
use xayn_web_api_shared::elastic::ScoreMap;

use super::{
    property_filter::{
        IndexedPropertiesSchema,
        IndexedPropertiesSchemaUpdate,
        IndexedPropertyDefinition,
        IndexedPropertyType,
    },
    utils::{Chunks, IterAsTuple, SqlBitCastU32},
    InteractionUpdateContext,
    TagWeights,
};
use crate::{
    ingestion::IngestionConfig,
    models::{
        DocumentContent,
        DocumentForIngestion,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentSnippet,
        DocumentTag,
        DocumentTags,
        ExcerptedDocument,
        PersonalizedDocument,
        Sha256Hash,
        SnippetForInteraction,
        SnippetId,
        SnippetOrDocumentId,
        UserId,
    },
    storage::{self, utils::SqlxPushTupleExt, KnnSearchParams, Storage, Warning},
    Error,
};

#[derive(FromRow)]
struct QueriedDeletedDocument {
    document_id: DocumentId,
    is_candidate: bool,
}

#[derive(FromRow)]
struct QueriedCoi {
    coi_id: CoiId,
    embedding: NormalizedEmbedding,
    /// The count is a `usize` stored as `i32` in database
    view_count: i32,
    /// The time is a `u64` stored as `i64` in database
    view_time_ms: i64,
    last_view: DateTime<Utc>,
}

impl Database {
    // https://docs.rs/sqlx/latest/sqlx/struct.QueryBuilder.html#note-database-specific-limits
    const BIND_LIMIT: usize = 65_535;

    pub(super) async fn insert_documents(
        &self,
        documents: &[DocumentForIngestion],
    ) -> Result<(), Error> {
        let mut tx = self.begin().await?;

        let mut builder = QueryBuilder::new(
            "INSERT INTO document (
                document_id,
                original_sha256,
                preprocessing_step,
                properties,
                tags,
                is_candidate
            ) ",
        );
        for chunk in documents.chunks(Self::BIND_LIMIT / 6) {
            builder
                .reset()
                .push_values(chunk, |mut builder, document| {
                    builder
                        .push_bind(&document.id)
                        .push_bind(&document.original_sha256)
                        .push_bind(document.preprocessing_step)
                        .push_bind(Json(&document.properties))
                        .push_bind(&document.tags)
                        .push_bind(document.is_candidate);
                })
                .push(
                    " ON CONFLICT (document_id) DO UPDATE SET
                        original_sha256 = EXCLUDED.original_sha256,
                        preprocessing_step = EXCLUDED.preprocessing_step,
                        properties = EXCLUDED.properties,
                        tags = EXCLUDED.tags,
                        is_candidate = EXCLUDED.is_candidate;",
                )
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await?;
        }

        let mut snippets = Chunks::new(
            Self::BIND_LIMIT / 4,
            documents.iter().flat_map(|document| {
                document.snippets.iter().enumerate().map(
                    |(sub_id, DocumentContent { snippet, embedding })| {
                        (
                            &document.id,
                            #[allow(clippy::cast_possible_truncation)]
                            SqlBitCastU32::from(sub_id as u32),
                            snippet,
                            embedding,
                        )
                    },
                )
            }),
        );

        let mut builder = QueryBuilder::new(
            "INSERT INTO snippet (
                        document_id,
                        sub_id,
                        snippet,
                        embedding
                    ) ",
        );

        while let Some(chunk) = snippets.next() {
            builder
                .reset()
                .push_values(
                    chunk,
                    |mut builder, (document_id, sub_id, snippet, embedding)| {
                        builder
                            .push_bind(document_id)
                            .push_bind(sub_id)
                            .push_bind(snippet)
                            .push_bind(embedding);
                    },
                )
                .push(
                    " ON CONFLICT (document_id, sub_id) DO UPDATE SET
                    snippet = EXCLUDED.snippet,
                    embedding = EXCLUDED.embedding;",
                )
                .build()
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn delete_documents(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<(Vec<DocumentId>, Warning<DocumentId>), Error> {
        let mut tx = self.begin().await?;

        let mut builder = QueryBuilder::new("DELETE FROM document WHERE document_id IN ");
        let all = ids.into_iter();
        let mut deleted = Vec::with_capacity(all.len());
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, all.clone());
        while let Some(ids) = chunks.next() {
            deleted.extend(
                builder
                    .reset()
                    .push_tuple(ids)
                    .push(" RETURNING document_id, is_candidate;")
                    .build()
                    .persistent(false)
                    .try_map(|row| QueriedDeletedDocument::from_row(&row))
                    .fetch_all(&mut tx)
                    .await?,
            );
        }

        tx.commit().await?;

        let failed = (deleted.len() < all.len())
            .then(|| {
                all.collect::<HashSet<_>>()
                    .difference(
                        &deleted
                            .iter()
                            .map(|document| &document.document_id)
                            .collect::<HashSet<_>>(),
                    )
                    .copied()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let candidates = deleted
            .into_iter()
            .filter_map(|document| document.is_candidate.then_some(document.document_id))
            .collect();

        Ok((candidates, failed))
    }

    async fn document_exists(
        tx: &mut Transaction<'_, Postgres>,
        id: &DocumentId,
    ) -> Result<bool, Error> {
        sqlx::query("SELECT FROM document WHERE document_id = $1;")
            .bind(id)
            .execute(tx)
            .await
            .map(|response| response.rows_affected() > 0)
            .map_err(Into::into)
    }

    async fn get_snippets_for_interaction(
        tx: &mut Transaction<'_, Postgres>,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
    ) -> Result<Vec<SnippetForInteraction>, Error> {
        let mut builder = QueryBuilder::new(
            "SELECT s.document_id, s.sub_id, s.embedding, d.tags
            FROM snippet s JOIN document d USING (document_id)
            WHERE (s.document_id, s.sub_id) IN ",
        );
        let mut chunks = IterAsTuple::chunks(
            Self::BIND_LIMIT,
            ids.into_iter()
                .map(|s| (s.document_id(), SqlBitCastU32::from(s.sub_id()))),
        );
        let mut documents = Vec::with_capacity(chunks.element_count());
        while let Some(ids) = chunks.next() {
            documents.extend(
                builder
                    .reset()
                    .push_nested_tuple(ids)
                    .build()
                    .try_map(|row: PgRow| {
                        let document_id = row.try_get("document_id")?;
                        let sub_id = row.try_get::<SqlBitCastU32, _>("sub_id")?;
                        let id = SnippetId::new(document_id, sub_id.into());
                        Ok(SnippetForInteraction {
                            id,
                            embedding: row.try_get("embedding")?,
                            tags: row.try_get("tags")?,
                        })
                    })
                    .fetch_all(&mut *tx)
                    .await?,
            );
        }

        Ok(documents)
    }

    async fn get_personalized(
        tx: &mut Transaction<'_, Postgres>,
        scores: ScoreMap<SnippetId>,
        include_properties: bool,
        include_snippet: bool,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut documents = Vec::with_capacity(scores.len());

        let mut builder = QueryBuilder::new(format!(
            "SELECT
                s.document_id, s.sub_id, s.embedding {snippet},
                d.tags {properties}
            FROM snippet s JOIN document d USING (document_id)
            WHERE (s.document_id, s.sub_id) IN ",
            properties = include_properties
                .then_some(", d.properties")
                .unwrap_or_default(),
            snippet = include_snippet.then_some(", s.snippet").unwrap_or_default(),
        ));
        let mut chunks = IterAsTuple::chunks(
            Self::BIND_LIMIT / 2,
            scores
                .keys()
                .map(|id| (id.document_id(), SqlBitCastU32::from(id.sub_id()))),
        );
        while let Some(ids) = chunks.next() {
            documents.extend(
                builder
                    .reset()
                    .push_nested_tuple(ids)
                    .build()
                    .try_map(|row: PgRow| {
                        let document_id = row.try_get("document_id")?;
                        let sub_id = u32::from(row.try_get::<SqlBitCastU32, _>("sub_id")?);
                        let id = SnippetId::new(document_id, sub_id);
                        let tags = row.try_get("tags")?;
                        let properties = if include_properties {
                            Some(row.try_get::<Json<_>, _>("properties")?.0)
                        } else {
                            None
                        };
                        let embedding = row.try_get("embedding")?;
                        let snippet = if include_snippet {
                            Some(row.try_get("snippet")?)
                        } else {
                            None
                        };

                        let score = scores[&id];

                        Ok(PersonalizedDocument {
                            id,
                            score,
                            embedding,
                            properties,
                            snippet,
                            tags,
                        })
                    })
                    .fetch_all(&mut *tx)
                    .await?,
            );
        }

        documents.sort_unstable_by(|d1, d2| d1.score.total_cmp(&d2.score).reverse());

        Ok(documents)
    }

    async fn get_excerpted(
        tx: &mut Transaction<'_, Postgres>,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<ExcerptedDocument>, Error> {
        let ids = ids.into_iter();

        let mut builder = QueryBuilder::new(
            "SELECT document_id, original_sha256, preprocessing_step, properties, tags, is_candidate
            FROM document
            WHERE document_id IN ",
        );
        let mut documents = Vec::with_capacity(ids.len());
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, ids);
        while let Some(ids) = chunks.next() {
            let chunk = builder
                .reset()
                .push_tuple(ids)
                .build()
                .try_map(|row: PgRow| {
                    Ok(ExcerptedDocument {
                        id: row.try_get("document_id")?,
                        original_sha256: row.try_get("original_sha256")?,
                        preprocessing_step: row.try_get("preprocessing_step")?,
                        properties: row.try_get::<Json<_>, _>("properties")?.0,
                        tags: row.try_get("tags")?,
                        is_candidate: row.try_get("is_candidate")?,
                    })
                })
                .fetch_all(&mut *tx)
                .await?;

            documents.extend(chunk);
        }

        Ok(documents)
    }

    async fn get_embedding(
        tx: &mut Transaction<'_, Postgres>,
        id: &SnippetId,
    ) -> Result<Option<NormalizedEmbedding>, Error> {
        sqlx::query_as("SELECT embedding FROM snippet WHERE document_id = $1 AND sub_id = $2;")
            .bind(id.document_id())
            .bind(SqlBitCastU32::from(id.sub_id()))
            .fetch_optional(tx)
            .await
            .map_err(Into::into)
    }

    async fn set_candidates(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<
        (
            HashSet<DocumentId>,
            Vec<DocumentForIngestion>,
            Warning<DocumentId>,
        ),
        Error,
    > {
        let mut tx = self.begin().await?;

        let mut ingestable = ids.into_iter().collect::<HashSet<_>>();
        let mut unchanged = HashSet::new();
        let mut removed = HashSet::new();

        sqlx::query_as::<_, (DocumentId,)>(
            "SELECT document_id
            FROM document
            WHERE is_candidate;",
        )
        .fetch(&mut tx)
        .try_for_each(|(document_id,)| {
            if ingestable.contains(&document_id) {
                unchanged.insert(document_id);
            } else {
                removed.insert(document_id);
            }
            future::ok(())
        })
        .await?;

        ingestable.retain(|id| !unchanged.contains(*id));

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = FALSE
            WHERE document_id IN ",
        );
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, removed.iter());
        while let Some(ids) = chunks.next() {
            builder
                .reset()
                .push_tuple(ids)
                .build()
                .execute(&mut tx)
                .await?;
        }

        let needs_ingestion =
            Self::set_is_candidate_and_return_for_ingestion(&mut tx, ingestable.iter().copied())
                .await?;

        tx.commit().await?;

        let failed = (needs_ingestion.len() < ingestable.len())
            .then(|| {
                for document in &needs_ingestion {
                    ingestable.remove(&document.id);
                }
                ingestable.into_iter().cloned().collect()
            })
            .unwrap_or_default();

        Ok((removed, needs_ingestion, failed))
    }

    async fn set_is_candidate_and_return_for_ingestion(
        tx: &mut Transaction<'_, Postgres>,
        ids: impl ExactSizeIterator<Item = &DocumentId> + Clone,
    ) -> Result<Vec<DocumentForIngestion>, Error> {
        let mut builder = QueryBuilder::new(
            "SELECT document_id, sub_id, snippet, embedding
            FROM snippet
            WHERE document_id IN ",
        );
        let mut snippets = HashMap::<_, Vec<_>>::new();
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, ids.clone());
        while let Some(ids) = chunks.next() {
            builder
                .reset()
                .push_tuple(ids)
                .build_query_as::<SqlSnippet>()
                .fetch(&mut *tx)
                .try_for_each(|snippet| {
                    snippets.entry(snippet.document_id).or_default().push((
                        u32::from(snippet.sub_id),
                        snippet.snippet,
                        snippet.embedding,
                    ));
                    future::ok(())
                })
                .await?;
        }

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = TRUE
            WHERE document_id IN ",
        );
        let mut needs_ingestion = Vec::with_capacity(ids.len());
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, ids);
        while let Some(ids) = chunks.next() {
            let chunk = builder
                .reset()
                .push_tuple(ids)
                .push(" RETURNING document_id, preprocessing_step, properties, tags;")
                .build()
                .try_map(|row: PgRow| {
                    let document_id = row.try_get("document_id")?;
                    //Hint: We currently assume there are no gaps.
                    //      I.e. if there are 10 snippets their sub ids are 0..10.
                    let snippets = snippets
                        .remove(&document_id)
                        .unwrap_or_default()
                        .into_iter()
                        .sorted_by_key(|(idx, _, _)| *idx)
                        .map(|(_, snippet, embedding)| DocumentContent { snippet, embedding })
                        .collect();

                    Ok(DocumentForIngestion {
                        id: document_id,
                        //FIXME clearly separate PG and ES
                        // we don't put raw document onto ES
                        original_sha256: Sha256Hash::zero(),
                        snippets,
                        preprocessing_step: row.try_get("preprocessing_step")?,
                        properties: row.try_get::<Json<_>, _>("properties")?.0,
                        tags: row.try_get("tags")?,
                        is_candidate: true,
                    })
                })
                .fetch_all(&mut *tx)
                .await?;

            needs_ingestion.extend(chunk);
        }

        Ok(needs_ingestion)
    }

    async fn add_candidates(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<(Vec<DocumentForIngestion>, Warning<DocumentId>), Error> {
        let mut tx = self.begin().await?;

        let mut ingestable = ids.into_iter().collect::<HashSet<_>>();
        let mut builder = QueryBuilder::new(
            "SELECT document_id
            FROM document
            WHERE is_candidate AND document_id IN ",
        );
        let mut unchanged = Vec::new();
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, ingestable.iter());
        while let Some(ids) = chunks.next() {
            unchanged.extend(
                builder
                    .reset()
                    .push_tuple(ids)
                    .build()
                    .try_map(|row| DocumentId::from_row(&row))
                    .fetch_all(&mut tx)
                    .await?,
            );
        }
        for id in &unchanged {
            ingestable.remove(id);
        }

        let needs_ingestion =
            Self::set_is_candidate_and_return_for_ingestion(&mut tx, ingestable.iter().copied())
                .await?;

        tx.commit().await?;

        let failed = (needs_ingestion.len() < ingestable.len())
            .then(|| {
                for document in &needs_ingestion {
                    ingestable.remove(&document.id);
                }
                ingestable.into_iter().cloned().collect()
            })
            .unwrap_or_default();

        Ok((needs_ingestion, failed))
    }

    async fn remove_candidates(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<(Vec<DocumentId>, Warning<DocumentId>), Error> {
        let mut tx = self.begin().await?;

        let mut removable = ids.into_iter().collect::<HashSet<_>>();
        let mut builder = QueryBuilder::new(
            "SELECT document_id
            FROM document
            WHERE NOT is_candidate AND document_id IN ",
        );
        let mut unchanged = Vec::new();
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, removable.iter());
        while let Some(ids) = chunks.next() {
            unchanged.extend(
                builder
                    .reset()
                    .push_tuple(ids)
                    .build()
                    .try_map(|row| DocumentId::from_row(&row))
                    .fetch_all(&mut tx)
                    .await?,
            );
        }
        for id in &unchanged {
            removable.remove(id);
        }

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = FALSE
            WHERE document_id IN ",
        );
        let mut removed = Vec::with_capacity(removable.len());
        let mut chunks = IterAsTuple::chunks(Self::BIND_LIMIT, removable.iter());
        while let Some(ids) = chunks.next() {
            removed.extend(
                builder
                    .reset()
                    .push_tuple(ids)
                    .push(" RETURNING document_id;")
                    .build()
                    .try_map(|row| DocumentId::from_row(&row))
                    .fetch_all(&mut tx)
                    .await?,
            );
        }

        tx.commit().await?;

        let failed = (removed.len() < removable.len())
            .then(|| {
                for id in &removed {
                    removable.remove(id);
                }
                removable.into_iter().cloned().collect()
            })
            .unwrap_or_default();

        Ok((removed, failed))
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
    ) -> Result<Vec<Coi>, Error> {
        sqlx::query_as::<_, QueriedCoi>(
            "SELECT coi_id, embedding, view_count, view_time_ms, last_view
            FROM center_of_interest
            WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(tx)
        .await
        .map(|interests| {
            interests
                .into_iter()
                .map(
                    // we convert it from usize/u64 to i32/i64 when we store it in the database
                    #[allow(clippy::cast_sign_loss)]
                    |coi| Coi {
                        id: coi.coi_id,
                        point: coi.embedding,
                        stats: CoiStats {
                            view_count: coi.view_count as usize,
                            view_time: Duration::from_millis(coi.view_time_ms as u64),
                            last_view: coi.last_view,
                        },
                    },
                )
                .collect()
        })
        .map_err(Into::into)
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
        cois: &HashMap<CoiId, Coi>,
    ) -> Result<(), Error> {
        let mut builder = QueryBuilder::new(
            "INSERT INTO center_of_interest (
                coi_id,
                user_id,
                embedding,
                view_count,
                view_time_ms,
                last_view
            ) ",
        );
        let mut iter = Chunks::new(Database::BIND_LIMIT / 6, cois.values());
        while let Some(chunk) = iter.next() {
            builder
                .reset()
                .push_values(chunk, |mut builder, update| {
                    // bit casting to signed int is fine as we fetch them as signed int before bit casting them back to unsigned int
                    // truncating to 64bit is fine as >292e+6 years is more then enough for this use-case
                    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
                    builder
                        .push_bind(update.id)
                        .push_bind(user_id)
                        .push_bind(&update.point)
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
        interactions: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
    ) -> Result<(), Error> {
        let mut interactions = Chunks::new(Database::BIND_LIMIT / 4, interactions);

        //FIXME micro benchmark and chunking+persist abstraction
        let persist = interactions.element_count() < 10;

        let mut builder = QueryBuilder::new(
            "INSERT INTO interaction (document_id, sub_id, user_id, time_stamp) ",
        );
        while let Some(chunk) = interactions.next() {
            builder
                .reset()
                .push_values(chunk, |mut builder, snippet_id| {
                    builder
                        .push_bind(snippet_id.document_id())
                        .push_bind(SqlBitCastU32::from(snippet_id.sub_id()))
                        .push_bind(user_id)
                        .push_bind(time);
                })
                .push(" ON CONFLICT DO NOTHING;")
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
        let mut builder = QueryBuilder::new("INSERT INTO weighted_tag (user_id, tag, weight) ");
        let mut updates = Chunks::new(Database::BIND_LIMIT / 3, updates);
        while let Some(updates) = updates.next() {
            builder
                .reset()
                .push_values(updates, |mut builder, (tag, weight_diff)| {
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

    async fn size_of_json(
        tx: &mut Transaction<'_, Postgres>,
        value: &Value,
    ) -> Result<usize, Error> {
        sqlx::query_as::<_, (i32,)>("SELECT pg_column_size($1);")
            .bind(Json(value))
            .fetch_one(tx)
            .await
            .map(
                #[allow(clippy::cast_sign_loss)]
                |size| size.0 as usize,
            )
            .map_err(Into::into)
    }
}

#[derive(FromRow)]
struct SqlSnippet {
    document_id: DocumentId,
    sub_id: SqlBitCastU32,
    snippet: DocumentSnippet,
    embedding: NormalizedEmbedding,
}

#[async_trait(?Send)]
impl storage::Document for Storage {
    async fn get_snippets_for_interaction(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
    ) -> Result<Vec<SnippetForInteraction>, Error> {
        let mut tx = self.postgres.begin().await?;
        let documents = Database::get_snippets_for_interaction(&mut tx, ids).await?;
        tx.commit().await?;

        Ok(documents)
    }

    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
        include_properties: bool,
        include_snippet: bool,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let ids = ids.into_iter().map(|id| (id.clone(), 1.0)).collect();
        let documents =
            Database::get_personalized(&mut tx, ids, include_properties, include_snippet).await?;
        tx.commit().await?;

        Ok(documents)
    }

    async fn get_excerpted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<ExcerptedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let documents = Database::get_excerpted(&mut tx, ids).await?;
        tx.commit().await?;

        Ok(documents)
    }

    #[instrument(skip(self))]
    async fn get_embedding(&self, id: &SnippetId) -> Result<Option<NormalizedEmbedding>, Error> {
        let mut tx = self.postgres.begin().await?;
        let embedding = Database::get_embedding(&mut tx, id).await?;
        tx.commit().await?;

        Ok(embedding)
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let include_properties = params.include_properties;
        let include_snippet = params.include_snippet;
        let scores = self.elastic.get_by_embedding(params).await?;
        let documents =
            Database::get_personalized(&mut tx, scores, include_properties, include_snippet)
                .await?;
        tx.commit().await?;

        Ok(documents)
    }

    async fn insert(
        &self,
        documents: Vec<DocumentForIngestion>,
    ) -> Result<Warning<DocumentId>, Error> {
        self.postgres.insert_documents(&documents).await?;
        let (candidates, noncandidates) = documents
            .into_iter()
            .partition_map::<Vec<_>, Vec<_>, _, _, _>(|document| {
                if document.is_candidate {
                    Either::Left(document)
                } else {
                    Either::Right(document.id)
                }
            });
        let failed_documents = self.elastic.upsert_documents(&candidates).await?;
        self.elastic.delete_by_parents(&noncandidates).await?;
        Ok(failed_documents)
    }

    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (candidates, failed_documents) = self.postgres.delete_documents(ids).await?;
        self.elastic.delete_by_parents(&candidates).await?;
        Ok(failed_documents)
    }
}

#[async_trait(?Send)]
impl storage::DocumentCandidate for Storage {
    async fn get(&self) -> Result<Vec<DocumentId>, Error> {
        sqlx::query_as("SELECT document_id FROM document WHERE is_candidate;")
            .fetch_all(&self.postgres)
            .await
            .map_err(Into::into)
    }

    async fn set(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (removed, ingested, mut failed) = self.postgres.set_candidates(ids).await?;
        self.elastic.delete_by_parents(&removed).await?;
        failed.extend(self.elastic.freshly_insert_documents(&ingested).await?);

        Ok(failed)
    }

    async fn add(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (ingested, mut failed) = self.postgres.add_candidates(ids).await?;
        failed.extend(self.elastic.freshly_insert_documents(&ingested).await?);

        Ok(failed)
    }

    async fn remove(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (removed, failed) = self.postgres.remove_candidates(ids).await?;
        self.elastic.delete_by_parents(&removed).await?;
        Ok(failed)
    }
}

#[async_trait]
impl storage::DocumentProperties for Storage {
    async fn get(&self, id: &DocumentId) -> Result<Option<DocumentProperties>, Error> {
        let mut tx = self.postgres.begin().await?;

        let properties = sqlx::query_as::<_, (Json<DocumentProperties>,)>(
            "SELECT properties
            FROM document
            WHERE document_id = $1;",
        )
        .bind(id)
        .fetch_optional(&mut tx)
        .await?
        .map(|properties| properties.0 .0);

        tx.commit().await?;

        Ok(properties)
    }

    async fn put(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        let mut tx = self.postgres.begin().await?;

        let inserted = sqlx::query_as::<_, (bool,)>(
            "UPDATE document
            SET properties = $1
            WHERE document_id = (
                SELECT document_id
                FROM document
                WHERE document_id = $2
                FOR UPDATE
            )
            RETURNING is_candidate;",
        )
        .bind(Json(properties))
        .bind(id)
        .fetch_optional(&mut tx)
        .await?;
        let inserted = if let Some((is_candidate,)) = inserted {
            if is_candidate {
                self.elastic
                    .insert_document_properties(id, properties)
                    .await?
            } else {
                Some(())
            }
        } else {
            None
        };

        tx.commit().await?;

        Ok(inserted)
    }

    async fn delete(&self, id: &DocumentId) -> Result<Option<()>, Error> {
        let mut tx = self.postgres.begin().await?;

        let deleted = sqlx::query_as::<_, (bool,)>(
            "UPDATE document
            SET properties = DEFAULT
            WHERE document_id = (
                SELECT document_id
                FROM document
                WHERE document_id = $1
                FOR UPDATE
            )
            RETURNING is_candidate;",
        )
        .bind(id)
        .fetch_optional(&mut tx)
        .await?;
        let deleted = if let Some((is_candidate,)) = deleted {
            if is_candidate {
                self.elastic.delete_document_properties(id).await?
            } else {
                Some(())
            }
        } else {
            None
        };

        tx.commit().await?;

        Ok(deleted)
    }
}

#[async_trait]
impl storage::DocumentProperty for Storage {
    async fn get(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<DocumentProperty>>, Error> {
        let mut tx = self.postgres.begin().await?;

        let property = sqlx::query_as::<_, (Json<DocumentProperty>,)>(
            "SELECT properties -> $1
            FROM document
            WHERE document_id = $2 AND properties ? $1;",
        )
        .bind(property_id)
        .bind(document_id)
        .fetch_optional(&mut tx)
        .await?;
        let property = if let Some(property) = property {
            Some(Some(property.0 .0))
        } else {
            Database::document_exists(&mut tx, document_id)
                .await?
                .then_some(None)
        };

        tx.commit().await?;

        Ok(property)
    }

    async fn put(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        let mut tx = self.postgres.begin().await?;

        let inserted = sqlx::query_as::<_, (bool,)>(
            "UPDATE document
            SET properties = jsonb_set(properties, $1, $2)
            WHERE document_id = (
                SELECT document_id
                FROM document
                WHERE document_id = $3
                FOR UPDATE
            )
            RETURNING is_candidate;",
        )
        .bind(slice::from_ref(property_id))
        .bind(Json(property))
        .bind(document_id)
        .fetch_optional(&mut tx)
        .await?;
        let inserted = if let Some((is_candidate,)) = inserted {
            if is_candidate {
                self.elastic
                    .insert_document_property(document_id, property_id, property)
                    .await?
            } else {
                Some(())
            }
        } else {
            None
        };

        tx.commit().await?;

        Ok(inserted)
    }

    async fn delete(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<()>>, Error> {
        let mut tx = self.postgres.begin().await?;

        let deleted = sqlx::query_as::<_, (bool,)>(
            "UPDATE document
            SET properties = properties - $1
            WHERE document_id = (
                SELECT document_id
                FROM document
                WHERE document_id = $2
                FOR UPDATE
            ) AND properties ? $1
            RETURNING is_candidate;",
        )
        .bind(property_id)
        .bind(document_id)
        .fetch_optional(&mut tx)
        .await?;
        let deleted = if let Some((is_candidate,)) = deleted {
            if is_candidate {
                self.elastic
                    .delete_document_property(document_id, property_id)
                    .await?
                    .map(|()| Some(()))
            } else {
                Some(Some(()))
            }
        } else {
            Database::document_exists(&mut tx, document_id)
                .await?
                .then_some(None)
        };

        tx.commit().await?;

        Ok(deleted)
    }
}

#[async_trait]
impl storage::Interest for Storage {
    async fn get(&self, user_id: &UserId) -> Result<Vec<Coi>, Error> {
        Database::get_user_interests(&self.postgres, user_id).await
    }
}

#[async_trait(?Send)]
impl storage::Interaction for Storage {
    async fn get(&self, user_id: &UserId) -> Result<Vec<DocumentId>, Error> {
        let mut tx = self.postgres.begin().await?;

        let documents = sqlx::query_as::<_, (DocumentId,)>(
            "SELECT DISTINCT document_id
            FROM interaction
            WHERE user_id = $1;",
        )
        .bind(user_id)
        .fetch(&mut tx)
        .map_ok(|(id,)| id)
        .try_collect()
        .await?;

        tx.commit().await?;

        Ok(documents)
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
        .execute(&self.postgres)
        .await?;

        Ok(())
    }

    async fn update_interactions(
        &self,
        user_id: &UserId,
        interactions: Vec<SnippetOrDocumentId>,
        store_user_history: bool,
        time: DateTime<Utc>,
        mut update_logic: impl for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> Coi,
    ) -> Result<(), Error> {
        let mut tx = self.postgres.begin().await?;
        Database::acquire_user_coi_lock(&mut tx, user_id).await?;

        // TODO[pmk/soon] proper support for interaction with multi-snippet documents
        let interactions = interactions
            .into_iter()
            .map(|id| match id {
                SnippetOrDocumentId::SnippetId(id) => id,
                SnippetOrDocumentId::DocumentId(id) => SnippetId::new(id, 0),
            })
            .collect_vec();

        let snippets = Database::get_snippets_for_interaction(&mut tx, interactions.iter()).await?;
        let snippet_map = snippets
            .iter()
            .map(|document| (&document.id, document))
            .collect::<HashMap<_, _>>();
        let mut tag_weight_diff = snippets
            .iter()
            .flat_map(|document| &document.tags)
            .map(|tag| (tag, 0))
            .collect::<HashMap<_, _>>();

        let mut interests = Database::get_user_interests(&mut tx, user_id).await?;
        let mut updates = HashMap::new();
        for document_id in interactions {
            if let Some(document) = snippet_map.get(&document_id) {
                let updated_coi = update_logic(InteractionUpdateContext {
                    document,
                    tag_weight_diff: &mut tag_weight_diff,
                    interests: &mut interests,
                    time,
                });
                // We might update the same coi min `interests` multiple times,
                // if we do we only want to keep the latest update.
                updates.insert(updated_coi.id, updated_coi);
            } else {
                info!(?document_id, "interacted snippet doesn't exist");
            }
        }

        Database::upsert_cois(&mut tx, user_id, time, &updates).await?;
        if store_user_history {
            Database::upsert_interactions(&mut tx, user_id, time, snippet_map.keys().copied())
                .await?;
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
    async fn get(&self, user_id: &UserId) -> Result<TagWeights, Error> {
        let mut tx = self.postgres.begin().await?;

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

    async fn put(
        &self,
        document_id: &DocumentId,
        tags: &DocumentTags,
    ) -> Result<Option<()>, Error> {
        let mut tx = self.postgres.begin().await?;

        let inserted = sqlx::query_as::<_, (bool,)>(
            "UPDATE document
            SET tags = $1
            WHERE document_id = (
                SELECT document_id
                FROM document
                WHERE document_id = $2
                FOR UPDATE
            )
            RETURNING is_candidate;",
        )
        .bind(tags)
        .bind(document_id)
        .fetch_optional(&mut tx)
        .await?;
        let inserted = if let Some((is_candidate,)) = inserted {
            if is_candidate {
                self.elastic.insert_document_tags(document_id, tags).await?
            } else {
                Some(())
            }
        } else {
            None
        };

        tx.commit().await?;

        Ok(inserted)
    }
}

#[async_trait(?Send)]
impl storage::Size for Storage {
    async fn json(&self, value: &Value) -> Result<usize, Error> {
        let mut tx = self.postgres.begin().await?;
        let size = Database::size_of_json(&mut tx, value).await?;
        tx.commit().await?;

        Ok(size)
    }
}

#[async_trait(?Send)]
impl storage::IndexedProperties for Storage {
    async fn load_schema(&self) -> Result<IndexedPropertiesSchema, Error> {
        let mut tx = self.postgres.begin().await?;
        let schema = Database::load_schema(&mut tx).await?;
        tx.commit().await?;
        Ok(schema)
    }

    async fn extend_schema(
        &self,
        update: IndexedPropertiesSchemaUpdate,
        ingestion_config: &IngestionConfig,
    ) -> Result<IndexedPropertiesSchema, Error> {
        let mut tx = self.postgres.begin().await?;
        let mut schema = Database::load_schema(&mut tx).await?;
        schema.update(update.clone(), ingestion_config.max_indexed_properties)?;
        Database::extend_postgres_schema(&mut tx, &update).await?;
        self.elastic
            .extend_mapping(&update, &ingestion_config.index_update)
            .await?;
        tx.commit().await?;
        Ok(schema)
    }
}

impl Database {
    async fn load_schema(
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<IndexedPropertiesSchema, Error> {
        let schema = sqlx::query_as::<_, (DocumentPropertyId, IndexedPropertyType)>(
            "SELECT name, type FROM indexed_property;",
        )
        .fetch_all(tx)
        .await?
        .into_iter()
        .map(|(id, r#type)| (id, IndexedPropertyDefinition { r#type }))
        .collect::<HashMap<_, _>>();

        Ok(schema.into())
    }

    async fn extend_postgres_schema(
        tx: &mut Transaction<'_, Postgres>,
        update: &IndexedPropertiesSchemaUpdate,
    ) -> Result<(), Error> {
        if update.len() == 0 {
            return Ok(());
        }
        QueryBuilder::new("INSERT INTO indexed_property(name, type) ")
            .push_values(update, |mut builder, (name, def)| {
                builder.push_bind(name).push_bind(def.r#type);
            })
            .build()
            .execute(tx)
            .await?;
        Ok(())
    }
}
