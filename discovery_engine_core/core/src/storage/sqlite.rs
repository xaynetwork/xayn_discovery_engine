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
use displaydoc::Display;
use num_traits::FromPrimitive;
use sqlx::{
    sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions},
    Pool,
    QueryBuilder,
};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_providers::Market;

use crate::document::{self, HistoricDocument};

use super::{
    models::{ApiDocumentView, NewsResource, NewscatcherData},
    FeedScope,
    Storage,
};

// Sqlite bind limit
const BIND_LIMIT: usize = 32766;

#[derive(Error, Debug, Display)]
pub enum DatabaseError {
    /// Failed to migrate database: {0}
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Sql error: {0}
    Sql(#[source] sqlx::Error),

    /// Failed to covert type: {0}
    TypeConversion(#[source] GenericError),
}

#[derive(Error, Debug, Display)]
pub enum StorageError {
    /// Database error: {0}
    Database(#[from] DatabaseError),
}

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub(crate) async fn connect(uri: &str) -> Result<Self, StorageError> {
        let opt = SqliteConnectOptions::from_str(uri)
            .map_err(DatabaseError::Sql)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(opt)
            .await
            .map_err(DatabaseError::Sql)?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    type StorageError = StorageError;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError> {
        sqlx::migrate!("src/storage/migrations")
            .run(&self.pool)
            .await
            .map_err(|err| DatabaseError::Migration(err).into())
    }

    async fn fetch_history(
        &self,
    ) -> Result<Vec<HistoricDocument>, <Self as Storage>::StorageError> {
        #[derive(sqlx::FromRow)]
        struct _HistoricDocument {
            document: Uuid,
            title: String,
            snippet: String,
            url: String,
        }

        let mut con = self.pool.acquire().await.map_err(DatabaseError::Sql)?;

        sqlx::query_as::<_, _HistoricDocument>(
            "SELECT nr.document, nr.title, nr.snippet, nr.url
                FROM HistoricDocument AS hd, NewsResource AS nr
                ON hd.document = nr.document;",
        )
        .fetch_all(&mut con)
        .await
        .map_err(DatabaseError::Sql)?
        .into_iter()
        .map(|hd| {
            let url = Url::parse(&hd.url).map_err(|e| DatabaseError::TypeConversion(e.into()))?;
            Ok(HistoricDocument {
                id: document::Id::from(hd.document),
                url,
                snippet: hd.snippet,
                title: hd.title,
            })
        })
        .collect()
    }

    fn feed(
        &self,
    ) -> &(dyn FeedScope<FeedScopeError = <Self as FeedScope>::FeedScopeError> + Send + Sync) {
        self
    }
}

#[derive(Error, Debug, Display)]
pub enum FeedScopeError {
    /// Database error: {0}
    Database(#[from] DatabaseError),
}

#[async_trait]
impl FeedScope for SqliteStorage {
    type FeedScopeError = FeedScopeError;

    async fn close_feed_document(
        &self,
        document: document::Id,
    ) -> Result<(), Self::FeedScopeError> {
        let mut con = self.pool.acquire().await.map_err(DatabaseError::Sql)?;
        sqlx::query("DELETE FROM FeedDocument WHERE document = ?;")
            .bind(document.as_uuid())
            .execute(&mut con)
            .await
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }

    async fn clear_feed(&self) -> Result<(), Self::FeedScopeError> {
        let mut con = self.pool.acquire().await.map_err(DatabaseError::Sql)?;
        sqlx::query("DELETE FROM FeedDocument;")
            .execute(&mut con)
            .await
            .map_err(DatabaseError::Sql)?;
        Ok(())
    }

    async fn fetch_feed(&self) -> Result<Vec<ApiDocumentView>, Self::FeedScopeError> {
        #[derive(sqlx::FromRow)]
        struct _ApiDocumentView {
            document: Uuid,
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

        let mut con = self.pool.acquire().await.map_err(DatabaseError::Sql)?;
        sqlx::query_as::<_, _ApiDocumentView>(
            "SELECT nr.document, nr.title, nr.snippet, nr.topic, nr.url, nr.image,
            nr.datePublished, nr.source, nr.market, nc.domainRank, nc.score, ur.userReaction,
            po.inBatchIndex
            FROM NewsResource as nr, NewscatcherData as nc, UserReaction as ur,
            FeedDocuments as fd, PresentationOrdering as po
            ON fd.document = nr.document, fd.document = nc.document,
            fd.document = ur.document, fd.document = po.document
            ORDERED BY po.timestamp, po.inBatchIndex ASC;",
        )
        .fetch_all(&mut con)
        .await
        .map_err(DatabaseError::Sql)?
        .into_iter()
        .map(|doc| {
            let url = Url::parse(&doc.url).map_err(|e| DatabaseError::TypeConversion(e.into()))?;
            let image = doc
                .image
                .map(|url| Url::parse(&url).map_err(|e| DatabaseError::TypeConversion(e.into())))
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
                document: document::Id::from(doc.document),
                news_resource,
                newscatcher_data,
                user_reacted,
                in_batch_index: doc.in_batch_index,
            })
        })
        .collect()
    }

    async fn store_documents(
        &self,
        documents: &[document::Document],
    ) -> Result<(), Self::FeedScopeError> {
        if documents.is_empty() {
            return Ok(());
        }

        // The amount of documents that we can store via bulk inserts is limited by
        // the sqlite bind limit.
        // bind_limit divided by the number of fields in the largest tuple (NewsResource)
        let documents = documents.iter().take(BIND_LIMIT / 9);

        let mut tx = self.pool.begin().await.map_err(DatabaseError::Sql)?;
        // Bulk inserts
        // https://docs.rs/sqlx-core/latest/sqlx_core/query_builder/struct.QueryBuilder.html#method.push_values

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
            .map_err(DatabaseError::Sql)?;

        // insert id into HistoricDocument table
        let mut query_builder = QueryBuilder::new("INSERT INTO HistoricDocument (document) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(DatabaseError::Sql)?;

        // insert data into NewsResource table
        let mut query_builder = QueryBuilder::new("INSERT INTO NewsResource (document, title, snippet, topic, url, image, datePublished, source, market) ");
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
            .map_err(DatabaseError::Sql)?;

        // insert data into NewscatcherData table
        let mut query_builder =
            QueryBuilder::new("INSERT INTO NewscatcherData (document, domainRank, score) ");
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
            .map_err(DatabaseError::Sql)?;

        // insert data into FeedDocument table
        let mut query_builder = QueryBuilder::new("INSERT INTO FeedDocument (document) ");
        query_builder.push_values(documents.clone(), |mut stm, doc| {
            stm.push_bind(doc.id.as_uuid());
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(DatabaseError::Sql)?;

        // insert data into PresentationOrdering table
        let timestamp = Utc::now();
        let mut query_builder = QueryBuilder::new(
            "INSERT INTO PresentationOrdering (document, timestamp, inBatchIndex) ",
        );
        query_builder.push_values(documents.enumerate(), |mut stm, (id, doc)| {
            stm.push_bind(doc.id.as_uuid())
                .push_bind(timestamp)
                .push_bind(id as u32);
        });
        query_builder
            .build()
            .persistent(false)
            .execute(&mut tx)
            .await
            .map_err(DatabaseError::Sql)?;

        tx.commit()
            .await
            .map_err(|err| DatabaseError::Sql(err).into())
    }
}

#[cfg(test)]
mod tests {
    use crate::{document::NewsResource, stack};

    use super::*;

    fn create_documents(n: u32) -> Vec<document::Document> {
        (0..n)
            .map(|i| {
                let title = format!("title-{}", i);
                let snippet = format!("snippet-{}", i);
                let url = Url::parse("http://example.com").unwrap();
                let id = document::Id::new();
                document::Document {
                    id,
                    stack_id: stack::Id::new_random(),
                    resource: NewsResource {
                        title,
                        snippet,
                        url,
                        ..Default::default()
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
            assert_eq!(history.url, doc.resource.url);
            assert_eq!(history.snippet, doc.resource.snippet);
            assert_eq!(history.title, doc.resource.title);
        });
    }
}
