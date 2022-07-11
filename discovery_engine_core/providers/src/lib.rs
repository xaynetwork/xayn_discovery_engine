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

//! Providers.

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod bing;
mod error;
mod helpers;
mod models;
mod newscatcher;

pub use models::{
    GenericArticle,
    HeadlinesQuery,
    NewsQuery,
    Rank,
    RankLimit,
    TrendingTopicsQuery,
    TrustedHeadlinesQuery,
    UrlWithDomain,
};
use std::sync::Arc;

pub use newscatcher::{
    Article as NewscatcherArticle,
    NewscatcherHeadlinesProvider,
    NewscatcherNewsProvider,
    NewscatcherTrustedHeadlinesProvider,
    Response as NewscatcherResponse,
};

pub use bing::{BingTrendingTopicsProvider, Response as BingResponse, TrendingTopic};

pub use error::Error;
pub use helpers::{
    clean_query::clean_query,
    filter::{Filter, Market},
    rest_endpoint::RestEndpoint,
};

use async_trait::async_trait;
use url::Url;

/// Abstraction over a provider for a news searching functionality.
#[async_trait]
pub trait NewsProvider: Send + Sync {
    async fn query_news(&self, query: &NewsQuery<'_>) -> Result<Vec<GenericArticle>, Error>;
}

/// Abstraction over a provider for the latest headlines.
#[async_trait]
pub trait HeadlinesProvider: Send + Sync {
    async fn query_headlines(
        &self,
        query: &HeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

/// Abstraction over a provider for headlines only from trusted sources.
#[async_trait]
pub trait TrustedHeadlinesProvider: Send + Sync {
    async fn query_trusted_sources(
        &self,
        query: &TrustedHeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

#[async_trait]
pub trait TrendingTopicsProvider: Send + Sync {
    // TODO: `TrendingTopic` here is the bing specific representation, which we don't really want to expose
    async fn query_trending_topics(
        &self,
        query: &TrendingTopicsQuery<'_>,
    ) -> Result<Vec<TrendingTopic>, Error>;
}

pub struct ProviderConfig {
    pub api_base_url: String,
    /// Key for accessing the API.
    pub api_key: String,
    /// Url path for the news search provider.
    pub news_provider_path: String,
    /// Url path for the latest headlines provider.
    pub headlines_provider_path: String,
}

pub struct Providers {
    pub headlines: Arc<dyn HeadlinesProvider>,
    pub trusted_headlines: Arc<dyn TrustedHeadlinesProvider>,
    pub news: Arc<dyn NewsProvider>,
    pub trending_topics: Arc<dyn TrendingTopicsProvider>,
}

fn create_endpoint_url(raw_base_url: &str, path: &str) -> Result<Url, Error> {
    let mut base_url = Url::parse(raw_base_url).map_err(|_| Error::MalformedUrlInConfig {
        url: raw_base_url.into(),
    })?;

    let mut segments = base_url
        .path_segments_mut()
        .map_err(|_| Error::MalformedUrlInConfig {
            url: raw_base_url.into(),
        })?;

    segments.pop_if_empty();
    let stripped_path = path.strip_prefix('/').unwrap_or(path);
    let stripped_path = stripped_path.strip_suffix('/').unwrap_or(stripped_path);

    for new_segment in stripped_path.split('/') {
        segments.push(new_segment);
        if new_segment.is_empty() {
            return Err(Error::MalformedUrlPathInConfig { path: path.into() });
        }
    }

    drop(segments);
    Ok(base_url)
}

fn select_provider<T: ?Sized>(
    endpoint: RestEndpoint,
    create_newscatcher: impl FnOnce(RestEndpoint) -> Arc<T>,
) -> Result<Arc<T>, Error> {
    if let Some(segments) = endpoint.url().path_segments() {
        for segment in segments {
            return match segment {
                "newscatcher" => Ok(create_newscatcher(endpoint)),
                _ => continue,
            };
        }
    }

    Err(Error::NoProviderForEndpoint {
        url: endpoint.url().to_string(),
    })
}

impl Providers {
    pub fn new(config: &ProviderConfig) -> Result<Self, Error> {
        let headlines_endpoint = RestEndpoint::new(
            create_endpoint_url(&config.api_base_url, &config.headlines_provider_path)?,
            config.api_key.clone(),
        ).with_get_as_post(true);

        let headlines = select_provider(
            headlines_endpoint,
            NewscatcherHeadlinesProvider::from_endpoint,
        )?;

        let news_endpoint = RestEndpoint::new(
            create_endpoint_url(&config.api_base_url, &config.news_provider_path)?,
            config.api_key.clone(),
        ).with_get_as_post(true);

        let news = select_provider(news_endpoint, NewscatcherNewsProvider::from_endpoint)?;

        // Note: Trusted-sources only works with newscatcher for now.
        let trusted_headlines_endpoint = RestEndpoint::new(
            create_endpoint_url(&config.api_base_url, "newscatcher/v2/trusted-sources")?,
            config.api_key.clone(),
        ).with_get_as_post(true);
        let trusted_headlines =
            NewscatcherTrustedHeadlinesProvider::from_endpoint(trusted_headlines_endpoint);

        // Note: Trending topics only works with bing for now.
        let trending_topics_endpoint = RestEndpoint::new(
            create_endpoint_url(&config.api_base_url, "bing/v1/trending-topics")?,
            config.api_key.clone(),
        );
        let trending_topics = BingTrendingTopicsProvider::from_endpoint(trending_topics_endpoint);

        Ok(Providers {
            headlines,
            trusted_headlines,
            news,
            trending_topics,
        })
    }
}
