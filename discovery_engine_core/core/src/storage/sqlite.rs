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
use chrono::{DateTime, Utc};
use num_traits::FromPrimitive;
use sqlx::{sqlite::Sqlite, FromRow, Pool, QueryBuilder, Transaction};
use url::Url;
use xayn_discovery_engine_providers::Market;

use crate::{
    document::{self, HistoricDocument, UserReaction, ViewMode},
    stack,
    storage::{
        models::{
            ApiDocumentView,
            NewDocument,
            NewsResource,
            NewscatcherData,
            Paging,
            Search,
            SearchBy,
            TimeSpentDocumentView,
        },
        utils::SqlxPushTupleExt,
        BoxedStorage,
        Error,
        FeedScope,
        FeedbackScope,
        InitDbHint,
        SearchScope,
        SourcePreferenceScope,
        SourceReactionScope,
        StateScope,
        Storage,
    },
    DartMigrationData,
};

use self::utils::SqlxSqliteResultExt;

mod dart_migrations;
mod setup;
mod utils;

// Sqlite bind limit
const BIND_LIMIT: usize = 32766;

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    #[allow(clippy::too_many_lines)]
    async fn store_new_documents(
        tx: &mut Transaction<'_, Sqlite>,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }
        let mut query_builder = QueryBuilder::new("INSERT INTO ");
        let timestamp = Utc::now();

        // The amount of documents that we can store via bulk inserts
        // (<https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values>)
        // is limited by the sqlite bind limit. Hence, the BIND_LIMIT is divided by the number of
        // fields in the largest tuple (NewsResource).
        for documents in documents.chunks(BIND_LIMIT / 9) {
            // insert id into Document table (FK of HistoricDocument)
            query_builder
                .reset()
                .push("Document (documentId) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(&doc.id);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;

            // insert id into HistoricDocument table
            query_builder
                .reset()
                .push("HistoricDocument (documentId) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(&doc.id);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;

            // insert data into NewsResource table
            query_builder
            .reset()
            .push("NewsResource (documentId, title, snippet, topic, url, image, datePublished, source, market) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id)
                    .push_bind(&doc.news_resource.title)
                    .push_bind(&doc.news_resource.snippet)
                    .push_bind(&doc.news_resource.topic)
                    .push_bind(doc.news_resource.url.as_str())
                    .push_bind(doc.news_resource.image.as_ref().map(Url::as_str))
                    .push_bind(&doc.news_resource.date_published)
                    .push_bind(&doc.news_resource.source)
                    .push_bind(format!(
                        "{}{}",
                        doc.news_resource.market.lang_code,
                        doc.news_resource.market.country_code,
                    ));
            })
            .build()
            .persistent(false)
            .execute(&mut *tx)
            .await?;

            // insert data into NewscatcherData table
            query_builder
                .reset()
                .push("NewscatcherData (documentId, domainRank, score) ")
                .push_values(documents, |mut stm, doc| {
                    // fine as we convert it back to u64 when we fetch it from the database
                    #[allow(clippy::cast_possible_wrap)]
                    stm.push_bind(&doc.id)
                        .push_bind(doc.newscatcher_data.domain_rank as i64)
                        .push_bind(doc.newscatcher_data.score);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;

            // insert data into PresentationOrdering table
            query_builder
                .reset()
                .push("PresentationOrdering (documentId, batchTimestamp, inBatchIndex) ")
                .push_values(documents.iter().enumerate(), |mut stm, (idx, doc)| {
                    // we won't have so many documents that idx > u32
                    #[allow(clippy::cast_possible_truncation)]
                    stm.push_bind(&doc.id)
                        .push_bind(timestamp.timestamp())
                        .push_bind(idx as u32);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;

            // insert data into Embedding table
            query_builder
                .reset()
                .push("Embedding (documentId, embedding) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(&doc.id).push_bind(doc.embedding.to_bytes());
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?;
        }

        Ok(())
    }

    async fn store_new_search_documents(
        tx: &mut Transaction<'_, Sqlite>,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        SqliteStorage::store_new_documents(tx, documents).await?;

        QueryBuilder::new("INSERT INTO SearchDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(tx)
            .await?;

        Ok(())
    }

    async fn get_document(
        tx: &mut Transaction<'_, Sqlite>,
        base_table: &'static str,
        id: document::Id,
    ) -> Result<ApiDocumentView, Error> {
        let document = sqlx::query_as::<_, QueriedApiDocumentView>(&format!(
            "SELECT
                documentId,
                nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market,
                nc.domainRank, nc.score,
                em.embedding,
                ur.userReaction,
                st.stackId
            FROM {base_table}
            JOIN NewsResource           AS nr   USING (documentId)
            JOIN NewscatcherData        AS nc   USING (documentId)
            JOIN PresentationOrdering   AS po   USING (documentId)
            JOIN Embedding              AS em   USING (documentId)
            LEFT JOIN UserReaction      AS ur   USING (documentId)
            LEFT JOIN StackDocument     AS st   USING (documentId)
            WHERE documentId = ?;",
        ))
        .bind(id)
        .fetch_one(tx)
        .await
        .on_row_not_found(Error::NoDocument(id))?;

        document.try_into()
    }

    async fn delete_documents_from<'a>(
        tx: &'a mut Transaction<'_, Sqlite>,
        ids: &'a [document::Id],
        table: &'static str,
    ) -> Result<bool, Error> {
        let mut deletion = false;
        if ids.is_empty() {
            return Ok(deletion);
        }

        let mut query_builder =
            QueryBuilder::new(format!("DELETE FROM {table} WHERE documentId IN "));
        for ids in ids.chunks(BIND_LIMIT) {
            deletion |= query_builder
                .reset()
                .push_tuple(ids)
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await?
                .rows_affected()
                > 0;
        }

        Ok(deletion)
    }

    async fn set_sources(
        &self,
        sources: &HashSet<String>,
        preference: SourcePreference,
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM SourcePreference WHERE preference = ?;")
            .bind(preference)
            .execute(&mut tx)
            .await?;

        if !sources.is_empty() {
            QueryBuilder::new("INSERT INTO SourcePreference(source, preference) ")
                .push_values(sources, |mut stm, source| {
                    stm.push_bind(source);
                    stm.push_bind(preference);
                })
                .push(" ON CONFLICT DO UPDATE SET preference = excluded.preference;")
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn fetch_sources(&self, preference: SourcePreference) -> Result<HashSet<String>, Error> {
        let mut tx = self.pool.begin().await?;

        let sources = sqlx::query_as::<_, Source>(
            "SELECT source
            FROM SourcePreference
            WHERE preference = ?;",
        )
        .bind(preference)
        .fetch_all(&mut tx)
        .await?
        .into_iter()
        .map(|s| s.0)
        .collect();

        tx.commit().await?;
        Ok(sources)
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn init_storage_system(
        file_path: Option<String>,
        dart_migration_data: Option<DartMigrationData>,
    ) -> Result<(BoxedStorage, InitDbHint), Error> {
        self::setup::init_storage_system(file_path.map(Into::into), dart_migration_data)
            .await
            .map(|(storage, hint)| (Box::new(storage) as _, hint))
    }

    async fn clear_database(&self) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;

        let deletion = sqlx::query("DELETE FROM Document;")
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0;
        let deletion = sqlx::query("DELETE FROM Search;")
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0
            || deletion;
        let deletion = sqlx::query("DELETE FROM SerializedState;")
            .execute(&mut tx)
            .await?
            .rows_affected()
            > 0
            || deletion;

        tx.commit().await?;

        Ok(deletion)
    }

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, Error> {
        let mut tx = self.pool.begin().await?;

        let documents = sqlx::query_as::<_, QueriedHistoricDocument>(
            "SELECT
                hd.documentId, nr.title, nr.snippet, nr.url
            FROM HistoricDocument   AS hd
            JOIN NewsResource       AS nr   USING (documentId);",
        )
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        documents.into_iter().map(TryInto::try_into).collect()
    }

    fn feed(&self) -> &(dyn FeedScope + Send + Sync) {
        self
    }

    fn search(&self) -> &(dyn SearchScope + Send + Sync) {
        self
    }

    fn feedback(&self) -> &(dyn FeedbackScope + Send + Sync) {
        self
    }

    fn state(&self) -> &(dyn StateScope + Send + Sync) {
        self
    }

    fn source_preference(&self) -> &(dyn SourcePreferenceScope + Send + Sync) {
        self
    }

    fn source_reaction(&self) -> &(dyn SourceReactionScope + Send + Sync) {
        self
    }
}

#[async_trait]
impl FeedScope for SqliteStorage {
    async fn delete_documents(&self, ids: &[document::Id]) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;

        let deletion = Self::delete_documents_from(&mut tx, ids, "FeedDocument").await?;

        tx.commit().await?;

        Ok(deletion)
    }

    async fn clear(&self) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;

        let deletion = sqlx::query("DELETE FROM FeedDocument;")
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(deletion.rows_affected() > 0)
    }

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Error> {
        let mut tx = self.pool.begin().await?;

        let documents = sqlx::query_as::<_, QueriedApiDocumentView>(
            "SELECT
                documentId,
                nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market,
                nc.domainRank, nc.score,
                em.embedding,
                ur.userReaction,
                st.stackId
            FROM FeedDocument
            JOIN NewsResource           AS nr   USING (documentId)
            JOIN NewscatcherData        AS nc   USING (documentId)
            JOIN PresentationOrdering   AS po   USING (documentId)
            JOIN Embedding              AS em   USING (documentId)
            JOIN StackDocument          As st   USING (documentId)
            LEFT JOIN UserReaction      AS ur   USING (documentId)
            ORDER BY po.batchTimestamp, po.inBatchIndex ASC;",
        )
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        documents.into_iter().map(TryInto::try_into).collect()
    }

    async fn store_documents(
        &self,
        documents: &[NewDocument],
        stack_ids: &HashMap<document::Id, stack::Id>,
    ) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        SqliteStorage::store_new_documents(&mut tx, documents).await?;

        // insert data into FeedDocument table
        let mut query_builder = QueryBuilder::new("INSERT INTO ");
        query_builder
            .push("FeedDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await?;

        query_builder
            .reset()
            .push("StackDocument (documentId, stackId) ")
            .push_values(stack_ids, |mut stm, (doc_id, stack_id)| {
                stm.push_bind(doc_id);
                stm.push_bind(stack_id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await?;

        tx.commit().await.map_err(Into::into)
    }
}

#[async_trait]
impl SearchScope for SqliteStorage {
    async fn store_new_search(
        &self,
        search: &Search,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO Search (rowid, searchBy, searchTerm, pageSize, pageNumber) VALUES (1, ?, ?, ?, ?)"
        )
            .bind(search.search_by as u8)
            .bind(&search.search_term)
            .bind(search.paging.size)
            .bind(search.paging.next_page)
            .execute(&mut tx)
            .await?;

        SqliteStorage::store_new_search_documents(&mut tx, documents).await?;

        tx.commit().await.map_err(Into::into)
    }

    async fn store_next_page(
        &self,
        page_number: u32,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        SqliteStorage::store_new_search_documents(&mut tx, documents).await?;

        sqlx::query("UPDATE Search SET pageNumber = ? WHERE rowid = 1;")
            .bind(page_number)
            .execute(&mut tx)
            .await?;

        tx.commit().await.map_err(Into::into)
    }

    async fn fetch(&self) -> Result<(Search, Vec<ApiDocumentView>), Error> {
        let mut tx = self.pool.begin().await?;

        let search = sqlx::query_as::<_, QueriedSearch>(
            "SELECT searchBy, searchTerm, pageNumber, pageSize
            FROM Search
            WHERE rowid = 1;",
        )
        .fetch_one(&mut tx)
        .await
        .on_row_not_found(Error::NoSearch)?;

        let documents = sqlx::query_as::<_, QueriedApiDocumentView>(
            "SELECT
                documentId,
                nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market,
                nc.domainRank, nc.score,
                em.embedding,
                ur.userReaction,
                NULL AS stackId
            FROM SearchDocument         AS sd
            JOIN NewsResource           AS nr   USING (documentId)
            JOIN NewscatcherData        AS nc   USING (documentId)
            JOIN PresentationOrdering   AS po   USING (documentId)
            JOIN Embedding              AS em   USING (documentId)
            LEFT JOIN UserReaction      AS ur   USING (documentId)
            ORDER BY po.batchTimestamp, po.inBatchIndex ASC;",
        )
        .fetch_all(&mut tx)
        .await?;

        tx.commit().await?;

        Ok((
            search.try_into()?,
            documents
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        ))
    }

    async fn clear(&self) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;

        // delete data from Document table where user reaction is neutral
        let ids = sqlx::query_as::<_, document::Id>(
            "SELECT sd.documentId
                FROM SearchDocument     AS sd
                LEFT JOIN UserReaction  AS ur   USING (documentId)
                WHERE ur.userReaction = ? OR ur.userReaction IS NULL;",
        )
        .bind(UserReaction::Neutral as u32)
        .fetch_all(&mut tx)
        .await?;

        Self::delete_documents_from(&mut tx, &ids, "Document").await?;

        // delete all remaining data from SearchDocument table
        sqlx::query("DELETE FROM SearchDocument;")
            .execute(&mut tx)
            .await?;

        // delete all data from Search table
        let deletion = sqlx::query("DELETE FROM Search;").execute(&mut tx).await?;

        tx.commit().await?;

        Ok(deletion.rows_affected() > 0)
    }

    /// Returns a document which then can be used to trigger a deep search.
    async fn get_document(&self, id: document::Id) -> Result<ApiDocumentView, Error> {
        let mut tx = self.pool.begin().await?;
        // Due to it's use-case it is not a problem to return a document which is no longer
        // in the search (we might even need this). Hence why we use `HistoricDocument`
        // as base table.
        let document = Self::get_document(&mut tx, "HistoricDocument", id).await?;
        tx.commit().await?;
        Ok(document)
    }
}

#[async_trait]
impl StateScope for SqliteStorage {
    async fn store(&self, bytes: &[u8]) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO SerializedState (rowid, state)
            VALUES (1, ?)
            ON CONFLICT DO UPDATE
            SET state = excluded.state;",
        )
        .bind(bytes)
        .execute(&mut tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn fetch(&self) -> Result<Option<Vec<u8>>, Error> {
        let mut tx = self.pool.begin().await?;

        let state =
            sqlx::query_as::<_, QueriedState>("SELECT state FROM SerializedState WHERE rowid = 1;")
                .fetch_optional(&mut tx)
                .await?;

        tx.commit().await?;

        Ok(state.and_then(|state| (!state.state.is_empty()).then(|| state.state)))
    }

    async fn clear(&self) -> Result<bool, Error> {
        let mut tx = self.pool.begin().await?;

        let deletion = sqlx::query("DELETE FROM SerializedState;")
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        tx.commit().await?;

        Ok(deletion.rows_affected() > 0)
    }
}

#[derive(FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueriedHistoricDocument {
    document_id: document::Id,
    title: String,
    snippet: String,
    url: String,
}

impl TryFrom<QueriedHistoricDocument> for HistoricDocument {
    type Error = Error;

    fn try_from(doc: QueriedHistoricDocument) -> Result<Self, Self::Error> {
        let url = Url::parse(&doc.url).map_err(|err| Error::Database(err.into()))?;
        Ok(HistoricDocument {
            id: doc.document_id,
            url,
            snippet: doc.snippet,
            title: doc.title,
        })
    }
}

#[derive(FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueriedApiDocumentView {
    document_id: document::Id,
    title: String,
    snippet: String,
    topic: String,
    url: String,
    image: Option<String>,
    date_published: DateTime<Utc>,
    source: String,
    market: String,
    domain_rank: i64,
    score: Option<f32>,
    embedding: Vec<u8>,
    user_reaction: Option<u32>,
    stack_id: Option<stack::Id>,
}

impl TryFrom<QueriedApiDocumentView> for ApiDocumentView {
    type Error = Error;

    fn try_from(doc: QueriedApiDocumentView) -> Result<Self, Self::Error> {
        let url = Url::parse(&doc.url).map_err(|err| Error::Database(err.into()))?;
        let image = doc
            .image
            .map(|url| Url::parse(&url).map_err(|err| Error::Database(err.into())))
            .transpose()?;
        let market = doc
            .market
            .is_char_boundary(2)
            .then(|| {
                let (lang, country) = doc.market.split_at(2);
                Market::new(lang, country)
            })
            .ok_or_else(|| {
                Self::Error::Database(format!("Failed to convert {} to Market", doc.market).into())
            })?;

        let news_resource = NewsResource {
            title: doc.title,
            snippet: doc.snippet,
            topic: doc.topic,
            url,
            image,
            date_published: doc.date_published,
            source: doc.source,
            market,
        };
        // fine as we convert it to i64 when we store it in the database
        #[allow(clippy::cast_sign_loss)]
        let newscatcher_data = NewscatcherData {
            domain_rank: doc.domain_rank as u64,
            score: doc.score,
        };

        Ok(ApiDocumentView {
            document_id: doc.document_id,
            news_resource,
            newscatcher_data,
            user_reaction: user_reaction_from_db(doc.user_reaction)?,
            embedding: doc.embedding.try_into()?,
            stack_id: doc.stack_id,
        })
    }
}

fn user_reaction_from_db(raw: Option<u32>) -> Result<Option<UserReaction>, Error> {
    raw.map(|value| {
        UserReaction::from_u32(value).ok_or_else(|| {
            Error::Database(format!("Failed to convert {value} to UserReaction").into())
        })
    })
    .transpose()
}

#[derive(FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueriedSearch {
    search_by: u32,
    search_term: String,
    page_number: u32,
    page_size: u32,
}

impl TryFrom<QueriedSearch> for Search {
    type Error = Error;

    fn try_from(search: QueriedSearch) -> Result<Self, Self::Error> {
        let search_by = SearchBy::from_u32(search.search_by).ok_or_else(|| {
            Error::Database(format!("Failed to convert {} to SearchBy", search.search_by).into())
        })?;

        Ok(Search {
            search_by,
            search_term: search.search_term,
            paging: Paging {
                size: search.page_size,
                next_page: search.page_number,
            },
        })
    }
}

#[async_trait]
impl FeedbackScope for SqliteStorage {
    async fn update_user_reaction(
        &self,
        document: document::Id,
        reaction: UserReaction,
    ) -> Result<ApiDocumentView, Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO UserReaction(documentId, userReaction) VALUES (?, ?)
                ON CONFLICT DO UPDATE SET userReaction = excluded.userReaction;",
        )
        .bind(document)
        .bind(reaction as u32)
        .execute(&mut tx)
        .await?;

        let document = Self::get_document(&mut tx, "FeedDocument", document).await?;

        tx.commit().await?;
        Ok(document)
    }

    async fn update_time_spent(
        &self,
        document: document::Id,
        view_mode: ViewMode,
        duration: Duration,
    ) -> Result<TimeSpentDocumentView, Error> {
        let view_time_ms = u32::try_from(duration.as_millis()).ok().unwrap_or(u32::MAX);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO ViewTimes(documentId, viewMode, viewTimeMs) VALUES (?, ?, ?)
                ON CONFLICT DO UPDATE SET viewTimeMs = viewTimeMs + excluded.viewTimeMs;",
        )
        .bind(document)
        .bind(view_mode as u32)
        .bind(view_time_ms)
        .execute(&mut tx)
        .await
        .on_fk_violation(Error::NoDocument(document))?;

        let view = sqlx::query_as::<_, QueryTimeSpentDocumentView>(
            "SELECT
                em.embedding,
                min(sum(vt.viewTimeMs), 4294967295) as aggregatedViewTime,
                ur.userReaction
            FROM Embedding          AS em
            JOIN ViewTimes          AS vt   USING (documentId)
            LEFT JOIN UserReaction  AS ur   USING (documentId)
            WHERE documentId = ?;",
        )
        .bind(document)
        .fetch_one(&mut tx)
        .await
        .on_row_not_found(Error::NoDocument(document))
        .and_then(TryInto::try_into)?;

        tx.commit().await?;

        Ok(view)
    }

    async fn update_source_reaction(&self, source: &str, liked: bool) -> Result<(), Error> {
        match self.fetch_source_reaction(source).await? {
            None => self.create_source_reaction(source, liked).await,
            Some(reaction) if reaction == liked => self.update_source_weight(source).await,
            _ => self.delete_source_reaction(source).await,
        }
    }
}

#[derive(FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueryTimeSpentDocumentView {
    embedding: Vec<u8>,
    aggregated_view_time: u32,
    user_reaction: Option<u32>,
}

impl TryFrom<QueryTimeSpentDocumentView> for TimeSpentDocumentView {
    type Error = Error;

    fn try_from(
        QueryTimeSpentDocumentView {
            embedding,
            aggregated_view_time,
            user_reaction,
        }: QueryTimeSpentDocumentView,
    ) -> Result<Self, Self::Error> {
        Ok(TimeSpentDocumentView {
            smbert_embedding: embedding.try_into()?,
            last_reaction: user_reaction_from_db(user_reaction)?,
            aggregated_view_time: Duration::from_millis(u64::from(aggregated_view_time)),
        })
    }
}

#[derive(Default, FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueriedState {
    state: Vec<u8>,
}

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[repr(i32)]
enum SourcePreference {
    Trusted = 0,
    Excluded = 1,
}

#[derive(FromRow)]
struct Source(String);

#[async_trait]
impl SourcePreferenceScope for SqliteStorage {
    async fn set_trusted(&self, sources: &HashSet<String>) -> Result<(), Error> {
        self.set_sources(sources, SourcePreference::Trusted).await
    }

    async fn set_excluded(&self, sources: &HashSet<String>) -> Result<(), Error> {
        self.set_sources(sources, SourcePreference::Excluded).await
    }

    async fn fetch_trusted(&self) -> Result<HashSet<String>, Error> {
        self.fetch_sources(SourcePreference::Trusted).await
    }

    async fn fetch_excluded(&self) -> Result<HashSet<String>, Error> {
        self.fetch_sources(SourcePreference::Excluded).await
    }
}

#[derive(FromRow)]
struct QueriedSourceReaction {
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    weight: i32,
    liked: bool,
}

#[async_trait]
impl SourceReactionScope for SqliteStorage {
    async fn fetch_source_reaction(&self, source: &str) -> Result<Option<bool>, Error> {
        let mut tx = self.pool.begin().await?;

        let reaction = sqlx::query_as::<_, QueriedSourceReaction>(
            "SELECT source, weight, liked
            FROM SourceReaction
            WHERE source = ?;",
        )
        .bind(source)
        .fetch_optional(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(reaction.map(|r| r.liked))
    }

    async fn create_source_reaction(&self, source: &str, liked: bool) -> Result<(), Error> {
        let weight = if liked { 1 } else { -1 };
        let last_updated = Utc::now().naive_utc();

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO SourceReaction (source, weight, lastUpdated, liked) VALUES (?, ?, ?, ?)",
        )
        .bind(source)
        .bind(weight)
        .bind(last_updated)
        .bind(liked)
        .execute(&mut tx)
        .await?;

        tx.commit().await.map_err(Into::into)
    }

    async fn update_source_weight(&self, source: &str) -> Result<(), Error> {
        let last_updated = Utc::now().naive_utc();

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE SourceReaction
            SET lastUpdated = ?,
                weight = CASE WHEN liked = 1 THEN (weight + 1) END
            WHERE source = ?",
        )
        .bind(last_updated)
        .bind(source)
        .execute(&mut tx)
        .await?;

        tx.commit().await.map_err(Into::into)
    }

    async fn delete_source_reaction(&self, source: &str) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM SourceReaction WHERE source = ?;")
            .bind(source)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use maplit::hashset;
    use std::{collections::HashSet, time::Duration};

    use xayn_discovery_engine_ai::Embedding;

    use crate::{document::NewsResource, stack, storage::models::NewDocument};

    use super::*;

    fn create_documents(n: u8) -> Vec<NewDocument> {
        (0..n)
            .map(|i| {
                document::Document {
                    id: document::Id::new(),
                    stack_id: stack::Id::nil(),
                    resource: NewsResource {
                        title: format!("title-{i}"),
                        snippet: format!("snippet-{i}"),
                        url: Url::parse(&format!("http://example-{i}.com")).unwrap(),
                        source_domain: format!("example-{i}.com"),
                        image: (i != 0)
                            .then(|| Url::parse(&format!("http://example-image-{i}.com")).unwrap()),
                        rank: u64::from(i),
                        score: (i != 0).then(|| f32::from(i)),
                        topic: format!("topic-{i}"),
                        ..NewsResource::default()
                    },
                    ..document::Document::default()
                }
                .into()
            })
            .collect()
    }

    fn stack_ids_for(doc: &[NewDocument], stack_id: stack::Id) -> HashMap<document::Id, stack::Id> {
        doc.iter().map(|doc| (doc.id, stack_id)).collect()
    }

    macro_rules! assert_eq_of_documents {
        ($left:expr, $right:expr) => {{
            let api_docs: &[ApiDocumentView] = &*$left;
            let docs: &[NewDocument] = &*$right;

            assert_eq!(api_docs.len(), docs.len());
            api_docs
                .iter()
                .enumerate()
                .zip(docs.iter())
                .for_each(|((idx, api_docs), doc)| {
                    assert_eq!(api_docs.document_id, doc.id);
                    assert_eq!(api_docs.news_resource, doc.news_resource);
                    assert_eq!(api_docs.newscatcher_data.domain_rank, idx as u64);
                })
        }};
    }

    impl SqliteStorage {
        async fn test_storage_system() -> BoxedStorage {
            SqliteStorage::init_storage_system(None, None)
                .await
                .unwrap()
                .0
        }
    }

    #[tokio::test]
    async fn test_fetch_history() {
        let storage = SqliteStorage::test_storage_system().await;
        let history = storage.fetch_history().await.unwrap();
        assert!(history.is_empty());

        let docs = create_documents(10);
        let stack_ids = stack_ids_for(&docs, stack::PersonalizedNews::id());
        storage
            .feed()
            .store_documents(&docs, &stack_ids)
            .await
            .unwrap();

        let history = storage.fetch_history().await.unwrap();
        assert_eq!(history.len(), docs.len());
        history.iter().zip(docs).for_each(|(history, doc)| {
            assert_eq!(history.id, doc.id);
            assert_eq!(history.title, doc.news_resource.title);
            assert_eq!(history.snippet, doc.news_resource.snippet);
            assert_eq!(history.url, doc.news_resource.url);
        });
    }

    #[tokio::test]
    async fn test_feed_methods() {
        let storage = SqliteStorage::test_storage_system().await;
        let feed = storage.feed().fetch().await.unwrap();
        assert!(feed.is_empty());

        let stack_id = stack::PersonalizedNews::id();
        let docs = create_documents(10);
        let stack_ids = stack_ids_for(&docs, stack_id);
        storage
            .feed()
            .store_documents(&docs, &stack_ids)
            .await
            .unwrap();

        let feed = storage.feed().fetch().await.unwrap();
        assert_eq_of_documents!(feed, &docs[..]);
        for doc in feed {
            assert_eq!(doc.stack_id, Some(stack_id));
        }

        assert!(storage
            .feed()
            .delete_documents(&[docs[0].id])
            .await
            .unwrap());
        let feed = storage.feed().fetch().await.unwrap();
        assert!(!feed.iter().any(|feed| feed.document_id == docs[0].id));

        assert!(storage.feed().clear().await.unwrap());
        let feed = storage.feed().fetch().await.unwrap();
        assert!(feed.is_empty());
    }

    #[tokio::test]
    async fn test_search_methods() {
        let storage = SqliteStorage::test_storage_system().await;
        let search = storage.search().fetch().await;
        assert!(search.is_err());
        assert!(!storage.search().clear().await.unwrap());

        let first_docs = create_documents(10);
        let new_search = Search {
            search_by: SearchBy::Query,
            search_term: "term".to_string(),
            paging: Paging {
                size: 100,
                next_page: 2,
            },
        };
        storage
            .search()
            .store_new_search(&new_search, &first_docs)
            .await
            .unwrap();

        let (search, search_docs) = storage.search().fetch().await.unwrap();
        assert_eq!(search, new_search);
        assert_eq_of_documents!(search_docs, first_docs);

        // FIXME the current design of PresentationOrdering has a problem:
        // if you in the same second add multiple batches of documents
        // they don't have a proper order anymore :=(
        tokio::time::sleep(Duration::from_secs(1)).await;

        let second_docs = create_documents(5);
        storage
            .search()
            .store_next_page(3, &second_docs)
            .await
            .unwrap();

        let (search, search_docs) = storage.search().fetch().await.unwrap();
        assert_eq!(
            search,
            Search {
                paging: Paging {
                    next_page: 3,
                    ..new_search.paging
                },
                ..new_search
            }
        );
        assert_eq_of_documents!(&search_docs[..10], first_docs);
        assert_eq_of_documents!(&search_docs[10..], second_docs);
        assert!(storage.search().clear().await.unwrap());
    }

    #[tokio::test]
    async fn test_empty_search() {
        let storage = SqliteStorage::test_storage_system().await;

        let new_search = Search {
            search_by: SearchBy::Query,
            search_term: "term".to_string(),
            paging: Paging {
                size: 100,
                next_page: 2,
            },
        };
        storage
            .search()
            .store_new_search(&new_search, &[])
            .await
            .unwrap();

        assert!(storage.search().fetch().await.unwrap().1.is_empty());
        assert!(storage.search().clear().await.unwrap());
    }

    #[tokio::test]
    async fn test_get_document() {
        let storage = super::setup::init_storage_system(None, None)
            .await
            .unwrap()
            .0;

        let id = document::Id::new();
        assert!(matches!(
            storage.search().get_document(id).await.unwrap_err(),
            Error::NoDocument(bad_id) if bad_id == id
        ));

        let documents = create_documents(1);
        let mut tx = storage.pool.begin().await.unwrap();
        SqliteStorage::store_new_documents(&mut tx, &documents)
            .await
            .unwrap();
        tx.commit().await.unwrap();

        let document = storage
            .search()
            .get_document(documents[0].id)
            .await
            .unwrap();
        assert_eq_of_documents!(&[document], documents);

        let id = document::Id::new();
        assert!(matches!(
            storage.search().get_document(id).await.unwrap_err(),
            Error::NoDocument(bad_id) if bad_id == id,
        ));
    }

    #[tokio::test]
    async fn test_rank_conversion() {
        let storage = SqliteStorage::test_storage_system().await;
        let mut docs = create_documents(1);
        docs[0].newscatcher_data.domain_rank = u64::MAX;
        storage
            .feed()
            .store_documents(&docs, &stack_ids_for(&docs, stack::PersonalizedNews::id()))
            .await
            .unwrap();
        let feed = storage.feed().fetch().await.unwrap();

        assert_eq!(
            feed[0].newscatcher_data.domain_rank,
            docs[0].newscatcher_data.domain_rank
        );
    }

    #[tokio::test]
    #[allow(clippy::similar_names)]
    async fn test_storing_user_reaction() {
        let storage = SqliteStorage::test_storage_system().await;
        let docs = create_documents(10);
        let stack_ids = stack_ids_for(&docs, stack::PersonalizedNews::id());
        storage
            .feed()
            .store_documents(&docs, &stack_ids)
            .await
            .unwrap();

        let doc0 = docs[0].id;
        let doc1 = docs[1].id;
        let doc2 = docs[2].id;
        let doc3 = docs[3].id;

        storage
            .feedback()
            .update_user_reaction(doc0, UserReaction::Positive)
            .await
            .unwrap();
        storage
            .feedback()
            .update_user_reaction(doc1, UserReaction::Negative)
            .await
            .unwrap();
        storage
            .feedback()
            .update_user_reaction(doc2, UserReaction::Neutral)
            .await
            .unwrap();
        storage
            .feedback()
            .update_user_reaction(doc3, UserReaction::Positive)
            .await
            .unwrap();
        storage
            .feedback()
            .update_user_reaction(doc3, UserReaction::Neutral)
            .await
            .unwrap();

        let feed = storage.feed().fetch().await.unwrap();
        assert_eq!(feed[0].document_id, doc0);
        assert_eq!(feed[0].user_reaction, Some(UserReaction::Positive));
        assert_eq!(feed[1].document_id, doc1);
        assert_eq!(feed[1].user_reaction, Some(UserReaction::Negative));
        assert_eq!(feed[2].document_id, doc2);
        assert_eq!(feed[2].user_reaction, Some(UserReaction::Neutral));
        assert_eq!(feed[3].document_id, doc3);
        assert_eq!(feed[3].user_reaction, Some(UserReaction::Neutral));
        for doc in &feed[4..] {
            assert_eq!(doc.user_reaction, None);
        }
    }

    #[tokio::test]
    async fn test_storing_user_reaction_returns_the_right_document() {
        let storage = SqliteStorage::test_storage_system().await;
        let docs = create_documents(1);
        let stack_ids = stack_ids_for(&docs, stack::PersonalizedNews::id());
        storage
            .feed()
            .store_documents(&docs, &stack_ids)
            .await
            .unwrap();

        let reacted_doc = storage
            .feedback()
            .update_user_reaction(docs[0].id, UserReaction::Positive)
            .await
            .unwrap();

        assert_eq_of_documents!(&[reacted_doc], docs);
    }

    #[tokio::test]
    async fn test_state() {
        let storage = SqliteStorage::test_storage_system().await;

        assert!(!storage.state().clear().await.unwrap());
        assert!(storage.state().fetch().await.unwrap().is_none());

        let state = (0..100).collect::<Vec<u8>>();
        storage.state().store(&state).await.unwrap();
        assert_eq!(storage.state().fetch().await.unwrap(), Some(state));

        let state = (100..=255).collect::<Vec<u8>>();
        storage.state().store(&state).await.unwrap();
        assert_eq!(storage.state().fetch().await.unwrap(), Some(state));

        assert!(storage.state().clear().await.unwrap());
    }

    #[tokio::test]
    async fn test_source_preference() {
        let storage = SqliteStorage::test_storage_system().await;

        // if no sources are set, return an empty set
        let trusted_db = storage.source_preference().fetch_trusted().await.unwrap();
        assert!(trusted_db.is_empty());
        let excluded_db = storage.source_preference().fetch_excluded().await.unwrap();
        assert!(excluded_db.is_empty());

        // set sources and fetch them
        // the sources fetched should match the sources previously set
        let trusted_sources = hashset! {"a".to_string(), "b".to_string()};
        let excluded_sources = hashset! {"c".to_string()};

        storage
            .source_preference()
            .set_trusted(&trusted_sources)
            .await
            .unwrap();
        storage
            .source_preference()
            .set_excluded(&excluded_sources)
            .await
            .unwrap();

        let trusted_db = storage.source_preference().fetch_trusted().await.unwrap();
        assert_eq!(trusted_db, trusted_sources);
        let excluded_db = storage.source_preference().fetch_excluded().await.unwrap();
        assert_eq!(excluded_db, excluded_sources);

        // set the excluded source "c" and so far unknown source "d" as new trusted sources
        // excluded sources should be empty
        // trusted sources should return {"d", "c"}
        let trusted_sources_upt = hashset! {"d".to_string(), "c".to_string()};
        storage
            .source_preference()
            .set_trusted(&trusted_sources_upt)
            .await
            .unwrap();
        let trusted_db = storage.source_preference().fetch_trusted().await.unwrap();
        assert_eq!(trusted_db, trusted_sources_upt);

        let excluded_db = storage.source_preference().fetch_excluded().await.unwrap();
        assert!(excluded_db.is_empty());

        let excluded_sources_upt = hashset! {"c".to_string()};
        storage
            .source_preference()
            .set_excluded(&excluded_sources_upt)
            .await
            .unwrap();
        let excluded_db = storage.source_preference().fetch_excluded().await.unwrap();
        assert_eq!(excluded_db, excluded_sources_upt);

        let trusted_db = storage.source_preference().fetch_trusted().await.unwrap();
        assert_eq!(trusted_db, hashset! {"d".to_string()});

        // unset all trusted sources
        storage
            .source_preference()
            .set_trusted(&HashSet::new())
            .await
            .unwrap();
        let trusted_db = storage.source_preference().fetch_trusted().await.unwrap();
        assert!(trusted_db.is_empty());
    }

    #[tokio::test]
    async fn test_store_time_spent() {
        let storage = SqliteStorage::test_storage_system().await;
        let docs = create_documents(3);
        let stack_ids = stack_ids_for(&docs, stack::PersonalizedNews::id());

        storage
            .feed()
            .store_documents(&docs, &stack_ids)
            .await
            .unwrap();

        storage
            .feedback()
            .update_user_reaction(docs[0].id, UserReaction::Positive)
            .await
            .unwrap();

        let view = storage
            .feedback()
            .update_time_spent(docs[0].id, ViewMode::Story, Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(
            view,
            TimeSpentDocumentView {
                smbert_embedding: Embedding::default(),
                last_reaction: Some(UserReaction::Positive),
                aggregated_view_time: Duration::from_secs(1),
            }
        );

        let view = storage
            .feedback()
            .update_time_spent(docs[1].id, ViewMode::Story, Duration::from_secs(3))
            .await
            .unwrap();

        assert_eq!(
            view,
            TimeSpentDocumentView {
                smbert_embedding: Embedding::default(),
                last_reaction: None,
                aggregated_view_time: Duration::from_secs(3),
            }
        );

        let view = storage
            .feedback()
            .update_time_spent(docs[1].id, ViewMode::Web, Duration::from_secs(7))
            .await
            .unwrap();

        assert_eq!(
            view,
            TimeSpentDocumentView {
                smbert_embedding: Embedding::default(),
                last_reaction: None,
                aggregated_view_time: Duration::from_secs(10),
            }
        );

        let view = storage
            .feedback()
            .update_time_spent(docs[1].id, ViewMode::Story, Duration::from_secs(13))
            .await
            .unwrap();

        assert_eq!(
            view,
            TimeSpentDocumentView {
                smbert_embedding: Embedding::default(),
                last_reaction: None,
                aggregated_view_time: Duration::from_secs(23),
            }
        );
    }
}
