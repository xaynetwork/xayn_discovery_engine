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

// Sqlite bind limit
const BIND_LIMIT: usize = 32766;

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub(crate) async fn connect(uri: &str) -> Result<Self, Error> {
        let opt = SqliteConnectOptions::from_str(uri)
            .map_err(|err| Error::Database(err.into()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(opt)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Ok(Self { pool })
    }

    async fn begin_tx(&self) -> Result<Transaction<'_, Sqlite>, Error> {
        self.pool
            .begin()
            .await
            .map_err(|err| Error::Database(err.into()))
    }

    async fn commit_tx(tx: Transaction<'_, Sqlite>) -> Result<(), Error> {
        tx.commit().await.map_err(|err| Error::Database(err.into()))
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
                .push("Document (id) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(&doc.id);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await
                .map_err(|err| Error::Database(err.into()))?;

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
                .await
                .map_err(|err| Error::Database(err.into()))?;

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
            .await
            .map_err(|err| Error::Database(err.into()))?;

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
                .await
                .map_err(|err| Error::Database(err.into()))?;

            // insert data into UserReaction table
            query_builder
                .reset()
                .push("UserReaction (documentId, userReaction) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(&doc.id)
                        .push_bind(UserReaction::default() as u32);
                })
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await
                .map_err(|err| Error::Database(err.into()))?;

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
                .await
                .map_err(|err| Error::Database(err.into()))?;
        }

        Ok(())
    }

    async fn clear_documents(
        tx: &mut Transaction<'_, Sqlite>,
        documents: &[document::Id],
    ) -> Result<bool, Error> {
        let mut deletion = false;
        if documents.is_empty() {
            return Ok(deletion);
        }

        let mut query_builder = QueryBuilder::new("DELETE FROM Document WHERE id IN (");
        for documents in documents.chunks(BIND_LIMIT) {
            let mut separated_builder = query_builder.reset().separated(", ");
            for id in documents {
                separated_builder.push_bind(id);
            }
            deletion |= query_builder
                .push(");")
                .build()
                .persistent(false)
                .execute(&mut *tx)
                .await
                .map_err(|err| Error::Database(err.into()))?
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
            .map_err(|err| Error::Database(err.into()))
    }

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, Error> {
        let mut tx = self.begin_tx().await?;

        let documents = sqlx::query_as::<_, QueriedHistoricDocument>(
            "SELECT
                nr.documentId, nr.title, nr.snippet, nr.url
            FROM
                HistoricDocument AS hd
            JOIN NewsResource AS nr ON hd.documentId = nr.documentId;",
        )
        .persistent(false)
        .fetch_all(&mut tx)
        .await
        .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await?;

        documents.into_iter().map(TryInto::try_into).collect()
    }

    fn feed(&self) -> &(dyn FeedScope + Send + Sync) {
        self
    }

    fn search(&self) -> &(dyn SearchScope + Send + Sync) {
        self
    }
}

#[async_trait]
impl FeedScope for SqliteStorage {
    async fn close_document(&self, document: &document::Id) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;

        sqlx::query("DELETE FROM FeedDocument WHERE documentId = ?;")
            .persistent(false)
            .bind(document)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }

    async fn clear(&self) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;

        sqlx::query("DELETE FROM FeedDocument;")
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Error> {
        let mut tx = self.begin_tx().await?;

        let documents = sqlx::query_as::<_, QueriedApiDocumentView>(
            "SELECT
                nr.documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                ur.userReaction, po.inBatchIndex
            FROM
                FeedDocument AS fd
            JOIN NewsResource AS nr ON fd.documentId = nr.documentId
            JOIN NewscatcherData AS nc ON fd.documentId = nc.documentId
            JOIN PresentationOrdering AS po ON fd.documentId = po.documentId
            JOIN UserReaction AS ur ON fd.documentId = ur.documentId
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
        )
        .persistent(false)
        .fetch_all(&mut tx)
        .await
        .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await?;

        documents.into_iter().map(TryInto::try_into).collect()
    }

    async fn store_documents(&self, documents: &[NewDocument]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let mut tx = self.begin_tx().await?;

        SqliteStorage::store_new_documents(&mut tx, documents).await?;

        // insert data into FeedDocument table
        QueryBuilder::new("INSERT INTO FeedDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }
}

#[async_trait]
impl SearchScope for SqliteStorage {
    async fn store_new_search(
        &self,
        search: &Search,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;
        let mut query_builder = QueryBuilder::new("INSERT INTO ");

        query_builder
            .push(
                "Search (rowid, searchBy, searchTerm, pageSize, pageNumber) VALUES (1, ?, ?, ?, ?)",
            )
            .build()
            .persistent(false)
            .bind(search.search_by as u8)
            .bind(&search.search_term)
            .bind(search.paging.size)
            .bind(search.paging.next_page)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        if documents.is_empty() {
            return Self::commit_tx(tx).await;
        };

        SqliteStorage::store_new_documents(&mut tx, documents).await?;
        query_builder
            .reset()
            .push("SearchDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }

    async fn store_next_page(
        &self,
        page_number: u32,
        documents: &[NewDocument],
    ) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;
        let mut query_builder = QueryBuilder::new(String::new());

        SqliteStorage::store_new_documents(&mut tx, documents).await?;
        query_builder
            .push("INSERT INTO SearchDocument (documentId) ")
            .push_values(documents, |mut stm, doc| {
                stm.push_bind(&doc.id);
            })
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;
        query_builder
            .reset()
            .push("UPDATE Search SET pageNumber = ? WHERE rowid = 1;")
            .build()
            .persistent(false)
            .bind(page_number)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }

    async fn fetch(&self) -> Result<(Search, Vec<ApiDocumentView>), Error> {
        let mut tx = self.begin_tx().await?;
        let mut query_builder = QueryBuilder::new("SELECT ");

        let search = query_builder
            .push(
                "searchBy, searchTerm, pageNumber, pageSize
            FROM Search
            WHERE rowid = 1;",
            )
            .build()
            .persistent(false)
            .try_map(|row| QueriedSearch::from_row(&row))
            .fetch_one(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        let documents = query_builder
            .reset()
            .push(
                "nr.documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                ur.userReaction, po.inBatchIndex
            FROM
                NewsResource AS nr, NewscatcherData AS nc, UserReaction AS ur,
                SearchDocument AS sd, PresentationOrdering AS po
            ON sd.documentId = nr.documentId
            AND sd.documentId = nc.documentId
            AND sd.documentId = ur.documentId
            AND sd.documentId = po.documentId
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
            )
            .build()
            .persistent(false)
            .try_map(|row| QueriedApiDocumentView::from_row(&row))
            .fetch_all(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await?;

        Ok((
            search.try_into()?,
            documents
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        ))
    }

    async fn clear(&self) -> Result<bool, Error> {
        let mut tx = self.begin_tx().await?;
        let mut query_builder = QueryBuilder::new(String::new());

        // delete data from Document table where user reaction is neutral
        let ids = query_builder
            .push(
                "SELECT ur.documentId
                FROM UserReaction AS ur, SearchDocument AS sd
                ON ur.documentId = sd.documentID
                WHERE ur.userReaction = ?;",
            )
            .build()
            .persistent(false)
            .bind(UserReaction::Neutral as u32)
            .try_map(|row| document::Id::from_row(&row))
            .fetch_all(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;
        Self::clear_documents(&mut tx, &ids).await?;

        // delete all remaining data from SearchDocument table
        query_builder
            .reset()
            .push("DELETE FROM SearchDocument;")
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // delete all data from Search table
        let deletion = query_builder
            .reset()
            .push("DELETE FROM Search;")
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await?;

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
    date_published: NaiveDateTime,
    source: String,
    market: String,
    domain_rank: i64,
    score: Option<f32>,
    user_reaction: Option<u32>,
    in_batch_index: u32,
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
                    Error::Database(format!("Failed to convert {value} to UserReaction",).into())
                })
            })
            .transpose()?;

        Ok(ApiDocumentView {
            document_id: doc.document_id,
            news_resource,
            newscatcher_data,
            user_reacted,
            in_batch_index: doc.in_batch_index,
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

#[cfg(test)]
mod tests {
    use crate::{document::NewsResource, stack, storage::models::NewDocument};

    use super::*;

    fn create_documents(n: u64) -> Vec<NewDocument> {
        (0..n)
            .map(|i| {
                document::Document {
                    id: document::Id::new(),
                    stack_id: stack::Id::new_random(),
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

    #[tokio::test]
    async fn test_fetch_history() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
        let history = storage.fetch_history().await.unwrap();
        assert!(history.is_empty());

        let docs = create_documents(10);
        storage.feed().store_documents(&docs).await.unwrap();

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
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();
        assert!(feed.is_empty());

        let docs = create_documents(10);
        storage.feed().store_documents(&docs).await.unwrap();

        let feed = storage.feed().fetch().await.unwrap();
        assert!(check_eq_of_documents(&feed, &docs));

        storage.feed().close_document(&docs[0].id).await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();
        assert!(!feed.iter().any(|feed| feed.document_id == docs[0].id));

        storage.feed().clear().await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();
        assert!(feed.is_empty());
    }

    #[tokio::test]
    async fn test_search_methods() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
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
    async fn test_rank_conversion() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
        let mut docs = create_documents(1);
        docs[0].newscatcher_data.domain_rank = u64::MAX;
        storage.feed().store_documents(&docs).await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();

        assert_eq!(
            feed[0].newscatcher_data.domain_rank,
            docs[0].newscatcher_data.domain_rank
        );
    }
}
