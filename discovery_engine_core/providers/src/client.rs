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

//! Client to get new documents.

use chrono::NaiveDateTime;
use displaydoc::Display as DisplayDoc;
use itertools::Itertools;
use serde::Deserialize;
use thiserror::Error;
use url::Url;

use crate::{
    gnews::Article as GnewsArticle,
    gnews_client::{Client as GnewsClient, NewsQuery as GnewsNewsQuery},
    newscatcher::Article as NewscatcherArticle,
    newscatcher_client::Client as NewscatcherClient,
    GnewsHeadlinesQuery,
    NewscatcherQuery,
};

/// Client errors.
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Invalid API Url base
    InvalidUrlBase(Option<url::ParseError>),
    /// Failed to execute the HTTP request: {0}
    RequestExecution(#[source] reqwest::Error),
    /// Server returned a non-successful status code: {0}
    StatusCode(#[source] reqwest::Error),
    /// Failed to fetch from the server: {0}
    Fetching(#[source] reqwest::Error),
    /// Failed to decode the server's response: {0}
    Decoding(#[source] serde_json::Error),
    /// Failed to decode the server's response at JSON path {1}: {0}
    DecodingAtPath(
        String,
        #[source] serde_path_to_error::Error<serde_json::Error>,
    ),
}

/// A news article
#[derive(Debug, Clone, Deserialize)]
pub struct Article {
    /// Title of the resource.
    pub title: String,

    /// Snippet of the resource.
    pub snippet: String,

    /// Url to reach the resource.
    pub url: String,

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub source_domain: String,

    /// Publishing date.
    pub date_published: NaiveDateTime,

    /// Image attached to the news.
    pub image: String,

    /// The rank of the domain of the source,
    pub rank: u64,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// Main topic of the publisher.
    pub topic: String,
}

impl Article {
    /// Gets the excerpt or falls back to the title if the excerpt is empty.
    pub fn snippet_or_title(&self) -> &str {
        (!self.snippet.is_empty())
            .then(|| &self.snippet)
            .unwrap_or(&self.title)
    }

    /// Loads newscatcher articles from JSON and returns them in `Article` representation
    pub fn load_from_newscatcher_json_representation(json: &str) -> Result<Vec<Self>, Error> {
        let articles: Vec<NewscatcherArticle> =
            serde_json::from_str(json).map_err(Error::Decoding)?;

        Ok(articles.into_iter().map(Into::into).collect())
    }
}

impl From<NewscatcherArticle> for Article {
    fn from(source: NewscatcherArticle) -> Self {
        Article {
            title: source.title,
            snippet: source.excerpt,
            url: source.link,
            source_domain: source.source_domain,
            date_published: source.published_date,
            image: source.media,
            rank: source.rank,
            score: source.score,
            country: source.country,
            language: source.language,
            topic: source.topic.to_string(),
        }
    }
}

impl From<GnewsArticle> for Article {
    fn from(source: GnewsArticle) -> Self {
        let source_domain = Url::parse(&source.url)
            .ok()
            .and_then(|url| url.domain().map(std::string::ToString::to_string))
            .unwrap_or_default();

        Article {
            title: source.title,
            snippet: source.description,
            url: source.url,
            source_domain,
            date_published: source.published_at.naive_local(),
            image: source.image,
            rank: 0,
            score: None,
            country: String::new(),
            language: String::new(),
            topic: String::new(),
        }
    }
}

/// Client that can provide documents.
pub struct Client {
    newscatcher: NewscatcherClient,
    gnews: GnewsClient,
}

impl Client {
    /// Create a client.
    pub fn new(token: impl Into<String>, url: impl Into<String>) -> Self {
        let token: String = token.into();
        let url: String = url.into();
        Self {
            newscatcher: NewscatcherClient::new(token.clone(), url.clone()),
            gnews: GnewsClient::new(token, url),
        }
    }

    /// Run a query for fetching `Article`s.
    pub async fn query_articles(&self, query: &GnewsNewsQuery<'_>) -> Result<Vec<Article>, Error> {
        self.gnews
            .query_articles(query)
            .await
            .map(|articles| articles.into_iter().map_into().collect())
    }

    /// Run a query for fetching `Article`s.
    pub async fn query_headlines(
        &self,
        query: &GnewsHeadlinesQuery<'_>,
    ) -> Result<Vec<Article>, Error> {
        self.gnews
            .query_headlines(query)
            .await
            .map(|articles| articles.into_iter().map_into().collect())
    }

    /// Run a query for fetching `Article`s from the newscatcher API.
    pub async fn query_newscatcher(
        &self,
        query: &impl NewscatcherQuery,
    ) -> Result<Vec<Article>, Error> {
        self.newscatcher
            .query_articles(query)
            .await
            .map(|articles| articles.into_iter().map_into().collect())
    }
}
