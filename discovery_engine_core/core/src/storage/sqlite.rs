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
use xayn_discovery_engine_providers::Market;

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

    async fn begin_tx(&self) -> Result<Transaction<'_, Sqlite>, Error> {
        self.pool
            .begin()
            .await
            .map_err(|err| Error::Database(err.into()))
    }

    async fn commit_tx(tx: Transaction<'_, Sqlite>) -> Result<(), Error> {
        tx.commit().await.map_err(|err| Error::Database(err.into()))
    }

    async fn store_new_documents<'a, I>(
        tx: &mut Transaction<'_, Sqlite>,
        documents: I,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = &'a NewDocument> + Send,
        <I as IntoIterator>::IntoIter: Clone,
    {
        let documents = documents.into_iter();
        if documents.clone().next().is_none() {
            return Ok(());
        }
        let mut query_builder = QueryBuilder::new("INSERT INTO ");

        // insert id into Document table (FK of HistoricDocument)
        query_builder
            .push("Document (id) ")
            .push_values(documents.clone(), |mut stm, doc| {
                stm.push_bind(doc.id.as_uuid());
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
            .push_values(documents.clone(), |mut stm, doc| {
                stm.push_bind(doc.id.as_uuid());
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
            .push_values(documents.clone(), |mut stm, doc| {
                stm.push_bind(doc.id.as_uuid())
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
            .push_values(documents.clone(), |mut stm, doc| {
                // fine as we convert it back to u64 when we fetch it from the database
                #[allow(clippy::cast_possible_wrap)]
                stm.push_bind(doc.id.as_uuid())
                    .push_bind(doc.newscatcher_data.domain_rank as i64)
                    .push_bind(doc.newscatcher_data.score);
            })
            .build()
            .persistent(false)
            .execute(tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Ok(())
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
                HistoricDocument AS hd, NewsResource AS nr
            ON hd.documentId = nr.documentId;",
        )
        .fetch_all(&mut tx)
        .await
        .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await?;

        documents.into_iter().map(TryInto::try_into).collect()
    }

    fn feed(&self) -> &(dyn FeedScope + Send + Sync) {
        self
    }
}

#[async_trait]
impl FeedScope for SqliteStorage {
    async fn close_document(&self, document: &document::Id) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;

        sqlx::query("DELETE FROM FeedDocument WHERE documentId = ?;")
            .bind(document.as_uuid())
            .execute(&mut tx)
            .await
            .map_err(|err| Error::Database(err.into()))?;

        Self::commit_tx(tx).await
    }

    async fn clear(&self) -> Result<(), Error> {
        let mut tx = self.begin_tx().await?;

        sqlx::query("DELETE FROM FeedDocument;")
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
                NewsResource AS nr, NewscatcherData AS nc, UserReaction AS ur,
                FeedDocument AS fd, PresentationOrdering AS po
            ON fd.documentId = nr.documentId
            AND fd.documentId = nc.documentId
            AND fd.documentId = ur.documentId
            AND fd.documentId = po.documentId
            ORDER BY po.timestamp, po.inBatchIndex ASC;",
        )
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
        let mut query_builder = QueryBuilder::new("INSERT INTO ");
        let timestamp = Utc::now();

        // The amount of documents that we can store via bulk inserts
        // (<https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values>)
        // is limited by the sqlite bind limit.
        // BIND_LIMIT divided by the number of fields in the largest tuple (NewsResource)
        for documents in documents.chunks(BIND_LIMIT / 9) {
            SqliteStorage::store_new_documents(&mut tx, documents).await?;

            // insert data into FeedDocument table
            query_builder
                .reset()
                .push("FeedDocument (documentId) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(doc.id.as_uuid());
                })
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await
                .map_err(|err| Error::Database(err.into()))?;

            // insert data into UserReaction table
            query_builder
                .reset()
                .push("UserReaction (documentId, userReaction) ")
                .push_values(documents, |mut stm, doc| {
                    stm.push_bind(doc.id.as_uuid())
                        .push_bind(UserReaction::default() as u32);
                })
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await
                .map_err(|err| Error::Database(err.into()))?;

            // insert data into PresentationOrdering table
            query_builder
                .reset()
                .push("PresentationOrdering (documentId, timestamp, inBatchIndex) ")
                .push_values(documents.iter().enumerate(), |mut stm, (idx, doc)| {
                    // we won't have so many documents that idx > u32
                    #[allow(clippy::cast_possible_truncation)]
                    stm.push_bind(doc.id.as_uuid())
                        .push_bind(timestamp)
                        .push_bind(idx as u32);
                })
                .build()
                .persistent(false)
                .execute(&mut tx)
                .await
                .map_err(|err| Error::Database(err.into()))?;
        }

        Self::commit_tx(tx).await
    }
}

#[derive(sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct QueriedHistoricDocument {
    document_id: Uuid,
    title: String,
    snippet: String,
    url: String,
}

impl TryFrom<QueriedHistoricDocument> for HistoricDocument {
    type Error = Error;

    fn try_from(doc: QueriedHistoricDocument) -> Result<Self, Self::Error> {
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
struct QueriedApiDocumentView {
    document_id: Uuid,
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
                #[allow(clippy::cast_precision_loss)]
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
