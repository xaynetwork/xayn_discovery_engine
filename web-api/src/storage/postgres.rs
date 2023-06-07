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
use itertools::Itertools;
use sqlx::{
    types::{
        chrono::{DateTime, Utc},
        Json,
    },
    Executor,
    FromRow,
    Postgres,
    QueryBuilder,
    Transaction,
};
use tracing::info;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{Coi, CoiId, CoiStats};

use super::{InteractionUpdateContext, TagWeights};
use crate::{
    models::{
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentTag,
        ExcerptedDocument,
        IngestedDocument,
        InteractedDocument,
        PersonalizedDocument,
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
struct QueriedInteractedDocument {
    document_id: DocumentId,
    tags: Vec<DocumentTag>,
    embedding: NormalizedEmbedding,
}

#[derive(FromRow)]
struct QueriedPersonalizedDocument {
    document_id: DocumentId,
    properties: Json<DocumentProperties>,
    tags: Vec<DocumentTag>,
    embedding: NormalizedEmbedding,
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
        documents: impl IntoIterator<Item = &IngestedDocument>,
    ) -> Result<(), Error> {
        let mut tx = self.begin().await?;

        let mut builder = QueryBuilder::new(
            "INSERT INTO document (
                document_id,
                snippet,
                properties,
                tags,
                embedding,
                is_candidate
            ) ",
        );
        let mut documents = documents.into_iter().peekable();
        while documents.peek().is_some() {
            builder
                .reset()
                .push_values(
                    documents.by_ref().take(Self::BIND_LIMIT / 6),
                    |mut builder, document| {
                        builder
                            .push_bind(&document.id)
                            .push_bind(&document.snippet)
                            .push_bind(Json(&document.properties))
                            .push_bind(&document.tags)
                            .push_bind(&document.embedding)
                            .push_bind(document.is_candidate);
                    },
                )
                .push(
                    " ON CONFLICT (document_id) DO UPDATE SET
                        snippet = EXCLUDED.snippet,
                        properties = EXCLUDED.properties,
                        tags = EXCLUDED.tags,
                        embedding = EXCLUDED.embedding,
                        is_candidate = EXCLUDED.is_candidate;",
                )
                .build()
                .persistent(false)
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
        let mut ids = all.clone().peekable();
        while ids.peek().is_some() {
            deleted.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
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
        sqlx::query("SELECT 1 FROM document WHERE document_id = $1;")
            .bind(id)
            .execute(tx)
            .await
            .map(|response| response.rows_affected() > 0)
            .map_err(Into::into)
    }

    async fn get_interacted(
        tx: &mut Transaction<'_, Postgres>,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<InteractedDocument>, Error> {
        let mut builder = QueryBuilder::new(
            "SELECT document_id, tags, embedding
            FROM document
            WHERE document_id IN ",
        );
        let mut ids = ids.into_iter().peekable();
        let mut documents = Vec::with_capacity(ids.len());
        while ids.peek().is_some() {
            documents.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .build()
                    .try_map(|row| {
                        QueriedInteractedDocument::from_row(&row).map(|document| {
                            InteractedDocument {
                                id: document.document_id,
                                embedding: document.embedding,
                                tags: document.tags,
                            }
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
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
        scores: impl Fn(&DocumentId) -> Option<f32> + Sync,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut builder = QueryBuilder::new(
            "SELECT document_id, properties, tags, embedding
            FROM document
            WHERE document_id IN ",
        );
        let ids = ids.into_iter();
        let mut documents = Vec::with_capacity(ids.len());
        let mut ids = ids.filter(|id| scores(id).is_some()).peekable();
        while ids.peek().is_some() {
            documents.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .build()
                    .try_map(|row| {
                        QueriedPersonalizedDocument::from_row(&row).map(|document| {
                            let score = scores(&document.document_id).unwrap(/* filtered ids */);
                            PersonalizedDocument {
                                id: document.document_id,
                                score,
                                embedding: document.embedding,
                                properties: document.properties.0,
                                tags: document.tags,
                            }
                        })
                    })
                    .fetch_all(&mut *tx)
                    .await?,
            );
        }
        documents.sort_unstable_by(|d1, d2| {
            scores(&d1.id)
                .unwrap()
                .total_cmp(&scores(&d2.id).unwrap())
                .reverse()
        });

        Ok(documents)
    }

    async fn get_excerpted(
        tx: &mut Transaction<'_, Postgres>,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<ExcerptedDocument>, Error> {
        let mut builder = QueryBuilder::new(
            "SELECT document_id, snippet, properties, tags, is_candidate
            FROM document
            WHERE document_id IN ",
        );
        let mut ids = ids.into_iter().peekable();
        let mut documents = Vec::with_capacity(ids.len());
        while ids.peek().is_some() {
            documents.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .build()
                    .try_map(|row| {
                        let (id, snippet, Json(properties), tags, is_candidate) =
                            FromRow::from_row(&row)?;
                        Ok(ExcerptedDocument {
                            id,
                            snippet,
                            properties,
                            tags,
                            is_candidate,
                        })
                    })
                    .fetch_all(&mut *tx)
                    .await?,
            );
        }

        Ok(documents)
    }

    async fn get_embedding(
        tx: &mut Transaction<'_, Postgres>,
        id: &DocumentId,
    ) -> Result<Option<NormalizedEmbedding>, Error> {
        sqlx::query_as("SELECT embedding FROM document WHERE document_id = $1;")
            .bind(id)
            .fetch_optional(tx)
            .await
            .map_err(Into::into)
    }

    async fn set_candidates(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<(Vec<DocumentId>, Vec<IngestedDocument>, Warning<DocumentId>), Error> {
        let mut tx = self.begin().await?;

        let mut ingestable = ids.into_iter().collect::<HashSet<_>>();
        let (unchanged, removed) = sqlx::query_as::<_, DocumentId>(
            "SELECT document_id
            FROM document
            WHERE is_candidate;",
        )
        .fetch_all(&mut tx)
        .await?
        .into_iter()
        .partition::<Vec<_>, _>(|id| ingestable.remove(id));

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = FALSE
            WHERE document_id IN ",
        );
        for ids in removed.chunks(Self::BIND_LIMIT) {
            builder
                .reset()
                .push_tuple(ids)
                .build()
                .execute(&mut tx)
                .await?;
        }

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = TRUE
            WHERE document_id IN ",
        );
        let mut ingested = Vec::with_capacity(ingestable.len());
        let mut ids = ingestable.iter().peekable();
        while ids.peek().is_some() {
            ingested.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .push(" RETURNING document_id, snippet, properties, tags, embedding;")
                    .build()
                    .try_map(|row| {
                        let (id, snippet, Json(properties), tags, embedding) =
                            FromRow::from_row(&row)?;
                        Ok(IngestedDocument {
                            id,
                            snippet,
                            properties,
                            tags,
                            embedding,
                            is_candidate: true,
                        })
                    })
                    .fetch_all(&mut tx)
                    .await?,
            );
        }

        tx.commit().await?;

        let failed = (ingested.len() < ingestable.len())
            .then(|| {
                for document in &ingested {
                    ingestable.remove(&document.id);
                }
                ingestable.into_iter().cloned().collect()
            })
            .unwrap_or_default();

        Ok((unchanged, ingested, failed))
    }

    async fn add_candidates(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<(Vec<IngestedDocument>, Warning<DocumentId>), Error> {
        let mut tx = self.begin().await?;

        let mut ingestable = ids.into_iter().collect::<HashSet<_>>();
        let mut builder = QueryBuilder::new(
            "SELECT document_id
            FROM document
            WHERE is_candidate AND document_id IN ",
        );
        let mut unchanged = Vec::new();
        let mut ids = ingestable.iter().peekable();
        while ids.peek().is_some() {
            unchanged.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .build()
                    .try_map(|row| DocumentId::from_row(&row))
                    .fetch_all(&mut tx)
                    .await?,
            );
        }
        for id in &unchanged {
            ingestable.remove(id);
        }

        let mut builder = QueryBuilder::new(
            "UPDATE document
            SET is_candidate = TRUE
            WHERE document_id IN ",
        );
        let mut ingested = Vec::with_capacity(ingestable.len());
        let mut ids = ingestable.iter().peekable();
        while ids.peek().is_some() {
            ingested.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
                    .push(" RETURNING document_id, snippet, properties, tags, embedding;")
                    .build()
                    .try_map(|row| {
                        let (id, snippet, Json(properties), tags, embedding) =
                            FromRow::from_row(&row)?;
                        Ok(IngestedDocument {
                            id,
                            snippet,
                            properties,
                            tags,
                            embedding,
                            is_candidate: true,
                        })
                    })
                    .fetch_all(&mut tx)
                    .await?,
            );
        }

        tx.commit().await?;

        let failed = (ingested.len() < ingestable.len())
            .then(|| {
                for document in &ingested {
                    ingestable.remove(&document.id);
                }
                ingestable.into_iter().cloned().collect()
            })
            .unwrap_or_default();

        Ok((ingested, failed))
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
        let mut ids = removable.iter().peekable();
        while ids.peek().is_some() {
            unchanged.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
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
        let mut ids = removable.iter().peekable();
        while ids.peek().is_some() {
            removed.extend(
                builder
                    .reset()
                    .push_tuple(ids.by_ref().take(Self::BIND_LIMIT))
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
        let mut iter = cois.values().peekable();
        while iter.peek().is_some() {
            builder
                .reset()
                .push_values(
                    iter.by_ref().take(Database::BIND_LIMIT / 6),
                    |mut builder, update| {
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
                    },
                )
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
        interactions: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<(), Error> {
        let mut interactions = interactions.into_iter().peekable();
        //FIXME micro benchmark and chunking+persist abstraction
        let persist = interactions.len() < 10;

        let mut builder =
            QueryBuilder::new("INSERT INTO interaction (doc_id, user_id, time_stamp) ");
        while interactions.peek().is_some() {
            builder
                .reset()
                .push_values(
                    interactions.by_ref().take(Database::BIND_LIMIT / 3),
                    |mut builder, document_id| {
                        builder
                            .push_bind(document_id)
                            .push_bind(user_id)
                            .push_bind(time);
                    },
                )
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
        let mut iter = updates.iter().peekable();
        while iter.peek().is_some() {
            builder
                .reset()
                .push_values(
                    iter.by_ref().take(Database::BIND_LIMIT / 3),
                    |mut builder, (tag, weight_diff)| {
                        builder
                            .push_bind(user_id)
                            .push_bind(tag)
                            .push_bind(weight_diff);
                    },
                )
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

#[async_trait(?Send)]
impl storage::Document for Storage {
    async fn get_interacted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<InteractedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let documents = Database::get_interacted(&mut tx, ids).await?;
        tx.commit().await?;

        Ok(documents)
    }

    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let documents = Database::get_personalized(&mut tx, ids, |_| Some(1.0)).await?;
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

    async fn get_embedding(&self, id: &DocumentId) -> Result<Option<NormalizedEmbedding>, Error> {
        let mut tx = self.postgres.begin().await?;
        let embedding = Database::get_embedding(&mut tx, id).await?;
        tx.commit().await?;

        Ok(embedding)
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let mut tx = self.postgres.begin().await?;
        let scores = self.elastic.get_by_embedding(params).await?;
        let documents =
            Database::get_personalized(&mut tx, scores.keys(), |id| scores.get(id).copied())
                .await?;
        tx.commit().await?;

        Ok(documents)
    }

    async fn insert(&self, documents: Vec<IngestedDocument>) -> Result<Warning<DocumentId>, Error> {
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
        let mut failed_documents = self.elastic.insert_documents(&candidates).await?;
        failed_documents.extend(self.elastic.delete_documents(&noncandidates).await?);

        Ok(failed_documents)
    }

    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (candidates, mut failed_documents) = self.postgres.delete_documents(ids).await?;
        failed_documents.extend(self.elastic.delete_documents(&candidates).await?);

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
        let (unchanged, ingested, mut failed) = self.postgres.set_candidates(ids).await?;
        self.elastic.retain_documents(&unchanged).await?;
        failed.extend(self.elastic.insert_documents(&ingested).await?);

        Ok(failed)
    }

    async fn add(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (ingested, mut failed) = self.postgres.add_candidates(ids).await?;
        failed.extend(self.elastic.insert_documents(&ingested).await?);

        Ok(failed)
    }

    async fn remove(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error> {
        let (removed, mut failed) = self.postgres.remove_candidates(ids).await?;
        failed.extend(self.elastic.delete_documents(&removed).await?);

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
        let mut tx = self.postgres.begin().await?;

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
        .execute(&self.postgres)
        .await?;

        Ok(())
    }

    async fn update_interactions(
        &self,
        user_id: &UserId,
        interactions: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
        store_user_history: bool,
        time: DateTime<Utc>,
        mut update_logic: impl for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> Coi,
    ) -> Result<(), Error> {
        let mut tx = self.postgres.begin().await?;
        Database::acquire_user_coi_lock(&mut tx, user_id).await?;

        let interactions = interactions.into_iter();
        let documents = Database::get_interacted(&mut tx, interactions.clone()).await?;
        let document_map = documents
            .iter()
            .map(|document| (&document.id, document))
            .collect::<HashMap<_, _>>();
        let mut tag_weight_diff = documents
            .iter()
            .flat_map(|document| &document.tags)
            .map(|tag| (tag, 0))
            .collect::<HashMap<_, _>>();

        let mut interests = Database::get_user_interests(&mut tx, user_id).await?;
        let mut updates = HashMap::new();
        for document_id in interactions {
            if let Some(document) = document_map.get(document_id) {
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
                info!(%document_id, "interacted document doesn't exist");
            }
        }

        Database::upsert_cois(&mut tx, user_id, time, &updates).await?;
        if store_user_history {
            Database::upsert_interactions(&mut tx, user_id, time, document_map.keys().copied())
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
        tags: &[DocumentTag],
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
