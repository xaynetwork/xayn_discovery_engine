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
use displaydoc::Display;
use sqlx::{
    sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions},
    Pool,
};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use xayn_discovery_engine_ai::GenericError;

use crate::document::{HistoricDocument, Id};

use super::Storage;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to initialize database: {0}
    Init(#[source] sqlx::Error),

    /// Failed to acquire connection: {0}
    AcquireConnection(#[source] sqlx::Error),

    /// Failed to migrate database: {0}
    Migration(#[from] sqlx::migrate::MigrateError),

    /// Failed to fetch: {0}
    Fetch(#[source] sqlx::Error),

    /// Failed to covert type: {0}
    TypeConversion(#[source] GenericError),
}

#[derive(Clone)]
pub(crate) struct SqliteStorage {
    pool: Pool<Sqlite>,
}

impl SqliteStorage {
    pub(crate) async fn connect(uri: &str) -> Result<Self, Error> {
        let opt = SqliteConnectOptions::from_str(uri)
            .map_err(Error::Init)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .connect_with(opt)
            .await
            .map_err(Error::Init)?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Storage for SqliteStorage {
    type StorageError = Error;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError> {
        sqlx::migrate!("src/storage/migrations")
            .run(&self.pool)
            .await
            .map_err(Into::into)
    }

    async fn fetch_history(
        &self,
    ) -> Result<Vec<HistoricDocument>, <Self as Storage>::StorageError> {
        #[derive(sqlx::FromRow)]
        struct _HistoricDocument {
            document: Uuid,
            url: String,
            snippet: String,
            title: String,
        }

        let mut con = self
            .pool
            .acquire()
            .await
            .map_err(Error::AcquireConnection)?;

        sqlx::query_as::<_, _HistoricDocument>("SELECT nr.document, nr.url, nr.snippet, nr.title FROM HistoricDocument AS hd, NewsResource AS nr ON hd.document = nr.document;")
            .fetch_all(&mut con)
            .await
            .map_err(Error::Fetch)?
            .into_iter()
            .map(|hd| {
                let url = Url::parse(&hd.url).map_err(|e| Error::TypeConversion(e.into()))?;
                Ok(HistoricDocument {
                    id: Id::from(hd.document),
                    url,
                    snippet: hd.snippet,
                    title: hd.title,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::document::NewsResource;

    use super::*;

    fn create_historic_documents(n: u32) -> Vec<(HistoricDocument, NewsResource)> {
        (0..n)
            .map(|i| {
                let title = format!("title-{}", i);
                let snippet = format!("snippet-{}", i);
                let url = Url::parse("http://example.com").unwrap();
                (
                    HistoricDocument {
                        id: Id::new(),
                        url: url.clone(),
                        snippet: snippet.clone(),
                        title: title.clone(),
                    },
                    NewsResource {
                        title,
                        snippet,
                        url,
                        ..Default::default()
                    },
                )
            })
            .collect()
    }

    async fn insert_historic_documents(
        storage: &SqliteStorage,
        docs: Vec<(HistoricDocument, NewsResource)>,
    ) {
        let mut con = storage.pool.acquire().await.unwrap();

        for doc in docs {
            sqlx::query("INSERT INTO Document (id) VALUES (?)")
                .bind(doc.0.id.as_uuid())
                .execute(&mut con)
                .await
                .unwrap();
            sqlx::query("INSERT INTO HistoricDocument (document) VALUES (?)")
                .bind(doc.0.id.as_uuid())
                .execute(&mut con)
                .await
                .unwrap();
            sqlx::query("INSERT INTO NewsResource (document, url, snippet, title, topic, datePublished, source) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(doc.0.id.as_uuid())
                .bind(doc.1.url.as_str())
                .bind(doc.1.snippet)
                .bind(doc.1.title)
                .bind(doc.1.topic)
                .bind(doc.1.date_published.to_string())
                .bind(doc.1.source_domain)
                .execute(&mut con)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_fetch_history() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();
        storage.init_database().await.unwrap();
        let history = storage.fetch_history().await.unwrap();
        assert!(history.is_empty());

        let docs = create_historic_documents(10);
        insert_historic_documents(&storage, docs.clone()).await;

        let history = storage.fetch_history().await.unwrap();
        assert_eq!(docs.len(), history.len());
        docs.iter().zip(history).for_each(|(a, b)| {
            assert_eq!(a.0.id, b.id);
            assert_eq!(a.1.url, b.url);
            assert_eq!(a.1.snippet, b.snippet);
            assert_eq!(a.1.title, b.title);
        });
    }
}
