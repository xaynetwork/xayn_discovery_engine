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
    Pool,
    QueryBuilder,
    Transaction,
};
use url::Url;
use uuid::Uuid;
use xayn_discovery_engine_ai::GenericError;

use crate::{
    document::{self, HistoricDocument, UserReaction},
    storage::{
        models::{ApiDocumentView, NewDocument, NewsResource, NewscatcherData},
        Error,
        FeedScope,
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

    async fn store_new_documents<'a>(
        mut tx: Transaction<'a, Sqlite>,
        documents: impl Iterator<Item = &NewDocument> + Clone + Send,
    ) -> Result<Transaction<'a, Sqlite>, Error> {
        // insert id into Document table (fk of HistoricDocument)
        let mut query_builder = QueryBuilder::new("INSERT INTO Document (id) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert id into HistoricDocument table
        let mut query_builder = QueryBuilder::new("INSERT INTO HistoricDocument (documentId) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert data into NewsResource table
        let mut query_builder = QueryBuilder::new("INSERT INTO NewsResource (documentId, title, snippet, topic, url, image, datePublished, source, market) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(&doc.news_resource.title)
                .push_bind(&doc.news_resource.snippet)
                .push_bind(&doc.news_resource.topic)
                .push_bind(doc.news_resource.url.to_string())
                .push_bind(doc.news_resource.image.as_ref().map(ToString::to_string))
                .push_bind(&doc.news_resource.date_published)
                .push_bind(&doc.news_resource.source)
                .push_bind(format!(
                    "{}{}",
                    doc.news_resource.market.country_code, doc.news_resource.market.lang_code,
                ));
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert data into NewscatcherData table
        let mut query_builder =
            QueryBuilder::new("INSERT INTO NewscatcherData (documentId, domainRank, score) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(doc.newscatcher_data.domain_rank)
                .push_bind(doc.newscatcher_data.score);
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;
        Ok(tx)
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
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| Error::Database(err.into()))?;

        sqlx::query_as::<_, _HistoricDocument>(
            "SELECT
                nr.documentId, nr.title, nr.snippet, nr.url
            FROM
                HistoricDocument AS hd, NewsResource AS nr
            ON hd.documentId = nr.documentId;",
        )
        .fetch_all(&mut con)
        .await
        .map_err(|err| Error::Database(err.into()))?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    fn feed(&self) -> &(dyn FeedScope + Send + Sync) {
        self
    }
}

#[async_trait]
impl FeedScope for SqliteStorage {
    async fn close_document(&self, document: &document::Id) -> Result<(), Error> {
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| Error::Database(err.into()))?;
        sqlx::query("DELETE FROM FeedDocument WHERE documentId = ?;")
            .bind(document.as_uuid())
            .execute(&mut con)
            .await
            .map_err(|err| Error::Database(err.into()))?;
        Ok(())
    }

    async fn clear(&self) -> Result<(), Error> {
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| Error::Database(err.into()))?;
        sqlx::query("DELETE FROM FeedDocument;")
            .execute(&mut con)
            .await
            .map_err(|err| Error::Database(err.into()))?;
        Ok(())
    }

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Error> {
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| Error::Database(err.into()))?;
        sqlx::query_as::<_, _ApiDocumentView>(
            "SELECT
                nr.documentId, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
                nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score,
                ur.userReaction, po.inBatchIndex
            FROM
                NewsResource AS nr, NewscatcherData AS nc, UserReaction AS ur,
                FeedDocument AS fd, PresentationOrdering AS po
            ON fd.documentId = nr.documentId
            AND fd.documentId = nc.documentId
            AND fd.documentId = ur.documentId
            AND fd.documentId = po.documentId
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
        )
        .fetch_all(&mut con)
        .await
        .map_err(|err| Error::Database(err.into()))?
        .into_iter()
        .map(TryInto::try_into)
        .collect()
    }

    async fn store_documents(&self, documents: &[NewDocument]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        // The amount of documents that we can store via bulk inserts
        // (https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values)
        // is limited by the sqlite bind limit.
        // bind_limit divided by the number of fields in the largest tuple (NewsResource)
        let documents = documents.iter().take(BIND_LIMIT / 9);

        // Begin transaction
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|err| Error::Database(err.into()))?;

        let mut tx = SqliteStorage::store_new_documents(tx, documents.clone())
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert data into FeedDocument table
        let mut query_builder = QueryBuilder::new("INSERT INTO FeedDocument (documentId) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert data into UserReaction table
        let mut query_builder =
            QueryBuilder::new("INSERT INTO UserReaction (documentId, userReaction) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(UserReaction::default() as u32);
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        // insert data into PresentationOrdering table
        let timestamp = Utc::now();
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO PresentationOrdering (documentId, timestamp, inBatchIndex) ",
        );
        query_builder.push_values(documents.enumerate(), |mut stm, (idx, doc)| {
            #[allow(clippy::cast_possible_truncation)]
            // we won't have so many documents that idx > u32
            stm.push_bind(doc.id.as_uuid())
                .push_bind(timestamp)
                .push_bind(idx as u32);
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        tx.commit().await.map_err(|err| Error::Database(err.into()))
    }
}

#[derive(sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct _HistoricDocument {
    document_id: Uuid,
    title: String,
    snippet: String,
    url: String,
}

impl TryFrom<_HistoricDocument> for HistoricDocument {
    type Error = Error;

    fn try_from(doc: _HistoricDocument) -> Result<Self, Self::Error> {
        let url = Url::parse(&doc.url).map_err(|err| Error::Database(err.into()))?;
        Ok(HistoricDocument {
            id: document::Id::from(doc.document_id),
            url,
            snippet: doc.snippet,
            title: doc.title,
        })
    }
}

#[derive(sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct _ApiDocumentView {
    document_id: Uuid,
    title: String,
    snippet: String,
    topic: String,
    url: String,
    image: Option<String>,
    date_published: NaiveDateTime,
    source: String,
    market: String,
    domain_rank: u32,
    score: Option<f32>,
    user_reaction: Option<u32>,
    in_batch_index: u32,
}

impl TryFrom<_ApiDocumentView> for ApiDocumentView {
    type Error = Error;

    fn try_from(doc: _ApiDocumentView) -> Result<Self, Self::Error> {
        let url = Url::parse(&doc.url).map_err(|err| Error::Database(err.into()))?;
        let image = doc
            .image
            .map(|url| Url::parse(&url).map_err(|err| Error::Database(err.into())))
            .transpose()?;
        let market = doc.market.split_at(2);

        let news_resource = NewsResource {
            title: doc.title,
            snippet: doc.snippet,
            topic: doc.topic,
            url,
            image,
            date_published: doc.date_published,
            source: doc.source,
            market: market.into(),
        };
        let newscatcher_data = NewscatcherData {
            domain_rank: doc.domain_rank,
            score: doc.score,
        };
        let user_reacted: Option<UserReaction> = doc
            .user_reaction
            .map(|value| {
                UserReaction::from_u32(value).ok_or_else(|| {
                    Error::Database(GenericError::from(format!(
                        "Failed to convert {} to UserReaction",
                        value
                    )))
                })
            })
            .transpose()?;

        Ok(ApiDocumentView {
            document_id: doc.document_id.into(),
            news_resource,
            newscatcher_data,
            user_reacted,
            in_batch_index: doc.in_batch_index,
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
                let id = document::Id::new();
                let title = format!("title-{}", i);
                let snippet = format!("snippet-{}", i);
                let url = Url::parse(&format!("http://example-{}.com", i)).unwrap();
                let source_domain = format!("example-{}.com", i);
                let image = Url::parse(&format!("http://example-image-{}.com", i)).unwrap();
                let topic = format!("topic-{}", i);
                #[allow(clippy::cast_precision_loss)]
                document::Document {
                    id,
                    stack_id: stack::Id::new_random(),
                    resource: NewsResource {
                        title,
                        snippet,
                        url,
                        source_domain,
                        image: (i != 0).then(|| image),
                        rank: i,
                        score: (i != 0).then(|| i as f32),
                        topic,
                        ..NewsResource::default()
                    },
                    ..document::Document::default()
                }
                .into()
            })
            .collect()
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
        assert_eq!(feed.len(), docs.len());
        #[allow(clippy::cast_possible_truncation)]
        feed.iter()
            .enumerate()
            .zip(docs.iter())
            .for_each(|((idx, feed), doc)| {
                assert_eq!(feed.document_id, doc.id);
                assert_eq!(feed.news_resource.title, doc.news_resource.title);
                assert_eq!(feed.news_resource.snippet, doc.news_resource.snippet);
                assert_eq!(feed.news_resource.topic, doc.news_resource.topic);
                assert_eq!(feed.news_resource.url, doc.news_resource.url);
                assert_eq!(feed.news_resource.image, doc.news_resource.image);
                assert_eq!(
                    feed.news_resource.date_published,
                    doc.news_resource.date_published
                );
                assert_eq!(feed.news_resource.source, doc.news_resource.source);
                assert_eq!(feed.news_resource.market, doc.news_resource.market);
                assert_eq!(
                    feed.newscatcher_data.domain_rank,
                    doc.newscatcher_data.domain_rank
                );
                assert_eq!(feed.newscatcher_data.score, doc.newscatcher_data.score);
                assert_eq!(feed.in_batch_index, idx as u32);
            });

        storage.feed().close_document(&docs[0].id).await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();
        assert!(!feed.iter().any(|feed| feed.document_id == docs[0].id));

        storage.feed().clear().await.unwrap();
        let feed = storage.feed().fetch().await.unwrap();
        assert!(feed.is_empty());
    }
}
