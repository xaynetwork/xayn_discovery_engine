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

use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use num_traits::FromPrimitive;
use sqlx::{
    sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions},
    FromRow,
    Pool,
    QueryBuilder,
    Transaction,
};
use url::Url;
use xayn_discovery_engine_providers::Market;

use crate::{
    document::{self, HistoricDocument, UserReaction},
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
        },
        Error,
        FeedScope,
        SearchScope,
        Storage,
    },
};

use self::utils::SqlxSqliteResultExt;
use super::FeedbackScope;
use crate::storage::utils::SqlxPushTupleExt;

mod utils;

// Sqlite bind limit
const BIND_LIMIT: usize = 32766;

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub(crate) async fn connect(uri: &str) -> Result<Self, Error> {
        let opt = SqliteConnectOptions::from_str(uri)?.create_if_missing(true);

        let pool = SqlitePoolOptions::new().connect_with(opt).await?;

        Ok(Self { pool })
    }

    async fn setup_stacks_sync(tx: &mut Transaction<'_, Sqlite>) -> Result<(), Error> {
        let expected_ids = &[
            stack::ops::breaking::BreakingNews::id(),
            stack::ops::personalized::PersonalizedNews::id(),
            stack::ops::trusted::TrustedNews::id(),
            stack::exploration::Stack::id(),
        ];

        let mut query_builder = QueryBuilder::new(String::new());
        query_builder
            .push("INSERT INTO Stack (stackId) ")
            .push_values(expected_ids, |mut stm, id| {
                stm.push_bind(id);
            })
            .push(" ON CONFLICT DO NOTHING;")
            .build()
            .persistent(false)
            .execute(&mut *tx)
            .await?;

        query_builder
            .reset()
            .push("DELETE FROM Stack WHERE stackId NOT IN ")
            .push_tuple(expected_ids)
            .build()
            .persistent(false)
            .execute(tx)
            .await?;
        Ok(())
    }

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
                .push("PresentationOrdering (documentId, timestamp, inBatchIndex) ")
                .push_values(documents.iter().enumerate(), |mut stm, (idx, doc)| {
                    // we won't have so many documents that idx > u32
                    #[allow(clippy::cast_possible_truncation)]
                    stm.push_bind(&doc.id)
                        .push_bind(timestamp)
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

    async fn get_document(
        tx: &mut Transaction<'_, Sqlite>,
        base_table: &'static str,
        id: document::Id,
    ) -> Result<ApiDocumentView, Error> {
        let document = sqlx::query_as::<_, QueriedApiDocumentView>(&format!(
            "SELECT
                documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                po.inBatchIndex, em.embedding, ur.userReaction, st.stackId
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
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn init_database(&self) -> Result<(), Error> {
        sqlx::migrate!("src/storage/migrations")
            .run(&self.pool)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        let mut tx = self.pool.begin().await?;
        Self::setup_stacks_sync(&mut tx).await?;
        tx.commit().await?;
        Ok(())
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
                fd.documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                ur.userReaction, po.inBatchIndex, em.embedding, st.stackId
            FROM FeedDocument           AS fd
            JOIN NewsResource           AS nr   USING (documentId)
            JOIN NewscatcherData        AS nc   USING (documentId)
            JOIN PresentationOrdering   AS po   USING (documentId)
            JOIN Embedding              AS em   USING (documentId)
            JOIN StackDocument          As st   USING (documentId)
            LEFT JOIN UserReaction      AS ur   USING (documentId)
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
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

        if documents.is_empty() {
            return tx.commit().await.map_err(Into::into);
        };

        SqliteStorage::store_new_documents(&mut tx, documents).await?;

        QueryBuilder::new("INSERT INTO SearchDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await?;

        tx.commit().await.map_err(Into::into)
    }

    async fn store_next_page(
        &self,
        page_number: u32,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        let mut tx = self.pool.begin().await?;

        SqliteStorage::store_new_documents(&mut tx, documents).await?;

        QueryBuilder::new("INSERT INTO SearchDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await?;

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
            "SELECT sd.documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                ur.userReaction, po.inBatchIndex, em.embedding, NULL AS stackId
            FROM SearchDocument         AS sd
            JOIN NewsResource           AS nr   USING (documentId)
            JOIN NewscatcherData        AS nc   USING (documentId)
            JOIN PresentationOrdering   AS po   USING (documentId)
            JOIN Embedding              AS em   USING (documentId)
            LEFT JOIN UserReaction      AS ur   USING (documentId)
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
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
    date_published: NaiveDateTime,
    source: String,
    market: String,
    domain_rank: i64,
    score: Option<f32>,
    user_reaction: Option<u32>,
    in_batch_index: u32,
    embedding: Vec<u8>,
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
        let user_reacted: Option<UserReaction> = doc
            .user_reaction
            .map(|value| {
                UserReaction::from_u32(value).ok_or_else(|| {
                    Error::Database(format!("Failed to convert {value} to UserReaction").into())
                })
            })
            .transpose()?;
        let embedding = doc.embedding.try_into().map_err(|err| {
            Error::Database(format!("Failed to convert bytes to Embedding: {err:?}").into())
        })?;

        Ok(ApiDocumentView {
            document_id: doc.document_id,
            news_resource,
            newscatcher_data,
            user_reacted,
            in_batch_index: doc.in_batch_index,
            embedding,
            stack_id: doc.stack_id,
        })
    }
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{document::NewsResource, stack, storage::models::NewDocument};

    use super::*;

    fn create_documents(n: u64) -> Vec<NewDocument> {
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
                        rank: i,
                        score: (i != 0).then(|| i as f32),
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

    fn check_eq_of_documents(api_docs: &[ApiDocumentView], docs: &[NewDocument]) -> bool {
        api_docs.len() == docs.len()
            && api_docs
                .iter()
                .enumerate()
                .zip(docs.iter())
                .all(|((idx, api_docs), doc)| {
                    api_docs.document_id == doc.id
                        && api_docs.news_resource == doc.news_resource
                        && api_docs.in_batch_index == idx as u32
                })
    }

    async fn create_memory_storage() -> impl Storage {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
        storage
    }

    #[tokio::test]
    async fn test_fetch_history() {
        let storage = create_memory_storage().await;
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
        let storage = create_memory_storage().await;
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
        assert!(check_eq_of_documents(&feed, &docs));
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
        let storage = create_memory_storage().await;
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
        assert!(check_eq_of_documents(&search_docs, &first_docs));

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
        assert!(check_eq_of_documents(&search_docs[..10], &first_docs));
        assert!(check_eq_of_documents(&search_docs[10..], &second_docs));
        assert!(storage.search().clear().await.unwrap());
    }

    #[tokio::test]
    async fn test_empty_search() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();

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
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();

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
        assert!(check_eq_of_documents(&[document], &documents));

        let id = document::Id::new();
        assert!(matches!(
            storage.search().get_document(id).await.unwrap_err(),
            Error::NoDocument(bad_id) if bad_id == id,
        ));
    }

    #[tokio::test]
    async fn test_rank_conversion() {
        let storage = create_memory_storage().await;
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
    async fn test_missing_stacks_are_added_and_removed_stacks_removed() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("src/storage/migrations")
            .run(&storage.pool)
            .await
            .unwrap();

        let mut tx = storage.pool.begin().await.unwrap();

        let random_id = stack::Id::new_random();
        sqlx::query("INSERT INTO Stack(stackId) VALUES (?), (?);")
            .bind(stack::PersonalizedNews::id())
            .bind(random_id)
            .execute(&mut tx)
            .await
            .unwrap();

        SqliteStorage::setup_stacks_sync(&mut tx).await.unwrap();

        //FIXME: For some reason if I try to read the stackIds from the database
        //       without first committing here the select statement will hang (sometimes).
        //       This happens even if no cached statements are used.
        tx.commit().await.unwrap();

        let ids = sqlx::query_as::<_, stack::Id>("SELECT stackId FROM Stack;")
            .fetch_all(&storage.pool)
            .await
            .unwrap();

        let expected_ids = [
            stack::ops::breaking::BreakingNews::id(),
            stack::ops::personalized::PersonalizedNews::id(),
            stack::ops::trusted::TrustedNews::id(),
            stack::exploration::Stack::id(),
        ];

        assert_eq!(
            ids.into_iter().collect::<HashSet<_>>(),
            expected_ids.into_iter().collect::<HashSet<_>>()
        );
    }

    #[tokio::test]
    async fn test_storing_user_reaction() {
        let storage = create_memory_storage().await;
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
        assert_eq!(feed[0].user_reacted, Some(UserReaction::Positive));
        assert_eq!(feed[1].document_id, doc1);
        assert_eq!(feed[1].user_reacted, Some(UserReaction::Negative));
        assert_eq!(feed[2].document_id, doc2);
        assert_eq!(feed[2].user_reacted, Some(UserReaction::Neutral));
        assert_eq!(feed[3].document_id, doc3);
        assert_eq!(feed[3].user_reacted, Some(UserReaction::Neutral));
        for doc in &feed[4..] {
            assert_eq!(doc.user_reacted, None);
        }
    }

    #[tokio::test]
    async fn test_storing_user_reaction_returns_the_right_document() {
        let storage = create_memory_storage().await;
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

        assert!(check_eq_of_documents(&[reacted_doc], &docs));
    }
}
