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
};
use url::Url;
use uuid::Uuid;
use xayn_discovery_engine_providers::Market;

use crate::{
    document::{self, HistoricDocument, UserReaction},
    storage::{
        self,
        models::{ApiDocumentView, NewsResource, NewscatcherData},
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
    pub(crate) async fn connect(uri: &str) -> Result<Self, storage::Error> {
        let opt = SqliteConnectOptions::from_str(uri)
            .map_err(|err| storage::Error::Database(err.into()))?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(opt)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn init_database(&self) -> Result<(), storage::Error> {
        sqlx::migrate!("src/storage/migrations")
            .run(&self.pool)
            .await
            .map_err(|err| storage::Error::Database(err.into()))
    }

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, storage::Error> {
        #[derive(sqlx::FromRow)]
        #[sqlx(rename_all = "camelCase")]
        struct _HistoricDocument {
            document_id: Uuid,
            title: String,
            snippet: String,
            url: String,
        }

        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

        sqlx::query_as::<_, _HistoricDocument>(
            "SELECT
                nr.documentId, nr.title, nr.snippet, nr.url
            FROM
                HistoricDocument AS hd, NewsResource AS nr
            ON hd.documentId = nr.documentId;",
        )
        .fetch_all(&mut con)
        .await
        .map_err(|err| storage::Error::Database(err.into()))?
        .into_iter()
        .map(|hd| {
            let url = Url::parse(&hd.url).map_err(|err| storage::Error::Database(err.into()))?;
            Ok(HistoricDocument {
                id: document::Id::from(hd.document_id),
                url,
                snippet: hd.snippet,
                title: hd.title,
            })
        })
        .collect()
    }

    fn feed(&self) -> &(dyn FeedScope + Send + Sync) {
        self
    }
}

#[async_trait]
impl FeedScope for SqliteStorage {
    async fn close_document(&self, document: &document::Id) -> Result<(), storage::Error> {
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;
        sqlx::query("DELETE FROM FeedDocument WHERE documentId = ?;")
            .bind(document.as_uuid())
            .execute(&mut con)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;
        Ok(())
    }

    async fn clear(&self) -> Result<(), storage::Error> {
        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;
        sqlx::query("DELETE FROM FeedDocument;")
            .execute(&mut con)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;
        Ok(())
    }

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, storage::Error> {
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
            score: f32,
            user_reaction: Option<u32>,
            in_batch_index: u32,
        }

        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;
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
        .map_err(|err| storage::Error::Database(err.into()))?
        .into_iter()
        .map(|doc| {
            let url = Url::parse(&doc.url).map_err(|err| storage::Error::Database(err.into()))?;
            let image = doc
                .image
                .map(|url| Url::parse(&url).map_err(|err| storage::Error::Database(err.into())))
                .transpose()?;
            let (lang_code, country_code) = doc.market.split_at(2);
            let market = Market {
                lang_code: lang_code.to_string(),
                country_code: country_code.to_string(),
            };

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
            let newscatcher_data = NewscatcherData {
                domain_rank: doc.domain_rank,
                score: Some(doc.score),
            };
            let user_reacted = doc.user_reaction.and_then(FromPrimitive::from_u32);

            Ok(ApiDocumentView {
                document_id: document::Id::from(doc.document_id),
                news_resource,
                newscatcher_data,
                user_reacted,
                in_batch_index: doc.in_batch_index,
            })
        })
        .collect()
    }

    #[allow(clippy::too_many_lines)]
    async fn store_documents(
        &self,
        documents: &[document::Document],
    ) -> Result<(), storage::Error> {
        if documents.is_empty() {
            return Ok(());
        }

        // The amount of documents that we can store via bulk inserts
        // (https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values)
        // is limited by the sqlite bind limit.
        // bind_limit divided by the number of fields in the largest tuple (NewsResource)
        let documents = documents.iter().take(BIND_LIMIT / 9);

        // Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

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
            .map_err(|err| storage::Error::Database(err.into()))?;

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
            .map_err(|err| storage::Error::Database(err.into()))?;

        // insert data into NewsResource table
        let mut query_builder = QueryBuilder::new("INSERT INTO NewsResource (documentId, title, snippet, topic, url, image, datePublished, source, market) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(&doc.resource.title)
                .push_bind(&doc.resource.snippet)
                .push_bind(&doc.resource.topic)
                .push_bind(doc.resource.url.to_string())
                .push_bind(doc.resource.image.as_ref().map(ToString::to_string))
                .push_bind(&doc.resource.date_published)
                .push_bind(&doc.resource.source_domain)
                .push_bind(format!("{}{}", doc.resource.language, doc.resource.country));
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

        // insert data into NewscatcherData table
        let mut query_builder =
            QueryBuilder::new("INSERT INTO NewscatcherData (documentId, domainRank, score) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(doc.resource.rank as u32)
                .push_bind(doc.resource.score.unwrap_or_default());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

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
            .map_err(|err| storage::Error::Database(err.into()))?;

        // insert data into UserReaction table
        let mut query_builder =
            QueryBuilder::new("INSERT INTO UserReaction (documentId, userReaction) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(UserReaction::Neutral as u32);
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(|err| storage::Error::Database(err.into()))?;

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
            .map_err(|err| storage::Error::Database(err.into()))?;

        tx.commit()
            .await
            .map_err(|err| storage::Error::Database(err.into()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{document::NewsResource, stack};

    use super::*;

    fn create_documents(n: u64) -> Vec<document::Document> {
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
            assert_eq!(history.title, doc.resource.title);
            assert_eq!(history.snippet, doc.resource.snippet);
            assert_eq!(history.url, doc.resource.url);
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
                assert_eq!(feed.news_resource.title, doc.resource.title);
                assert_eq!(feed.news_resource.snippet, doc.resource.snippet);
                assert_eq!(feed.news_resource.topic, doc.resource.topic);
                assert_eq!(feed.news_resource.url, doc.resource.url);
                assert_eq!(feed.news_resource.image, doc.resource.image);
                assert_eq!(
                    feed.news_resource.date_published,
                    doc.resource.date_published
                );
                assert_eq!(feed.news_resource.source, doc.resource.source_domain);
                assert_eq!(
                    feed.news_resource.market,
                    (&doc.resource.country, &doc.resource.language).into()
                );
                assert_eq!(feed.newscatcher_data.domain_rank, doc.resource.rank as u32);
                assert_eq!(
                    feed.newscatcher_data.score,
                    doc.resource.score.or(Some(0.0))
                );
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
