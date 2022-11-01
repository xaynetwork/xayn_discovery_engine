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

mod config;
mod error;
mod helpers;
mod mlt;
mod models;
mod newscatcher;

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;

pub use crate::{
    config::Config,
    error::Error,
    helpers::{
        clean_query::clean_query,
        filter::{Filter, Market},
        rest_endpoint::RestEndpoint,
    },
    mlt::MltSimilarNewsProvider,
    models::{
        GenericArticle,
        HeadlinesQuery,
        NewsQuery,
        Rank,
        RankLimit,
        SimilarNewsQuery,
        TrustedHeadlinesQuery,
        UrlWithDomain,
    },
    newscatcher::{
        Article as NewscatcherArticle,
        NewscatcherHeadlinesProvider,
        NewscatcherNewsProvider,
        NewscatcherTrustedHeadlinesProvider,
        Response as NewscatcherResponse,
    },
};

/// Provider for news search functionality.
#[async_trait]
pub trait NewsProvider: Send + Sync {
    async fn query_news(&self, query: &NewsQuery<'_>) -> Result<Vec<GenericArticle>, Error>;
}

/// Provider for the latest headlines.
#[async_trait]
pub trait HeadlinesProvider: Send + Sync {
    async fn query_headlines(
        &self,
        query: &HeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

/// Provider for headlines only from trusted sources.
#[async_trait]
pub trait TrustedHeadlinesProvider: Send + Sync {
    async fn query_trusted_sources(
        &self,
        query: &TrustedHeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

/// Provider for similar news.
#[async_trait]
pub trait SimilarNewsProvider: Send + Sync {
    async fn query_similar_news(
        &self,
        query: &SimilarNewsQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

pub struct Providers {
    pub headlines: Arc<dyn HeadlinesProvider>,
    pub trusted_headlines: Arc<dyn TrustedHeadlinesProvider>,
    pub news: Arc<dyn NewsProvider>,
    pub similar_news: Arc<dyn SimilarNewsProvider>,
}

fn select_provider<T: ?Sized>(
    endpoint: RestEndpoint,
    create_newscatcher: impl FnOnce(RestEndpoint) -> Arc<T>,
) -> Result<Arc<T>, Error> {
    if let Some(segments) = endpoint.config.url.path_segments() {
        for segment in segments {
            return match segment {
                "newscatcher" => Ok(create_newscatcher(endpoint)),
                _ => continue,
            };
        }
    }

    Err(Error::NoProviderForEndpoint {
        url: endpoint.config.url.to_string(),
    })
}

impl Providers {
    pub fn new(
        api_base_url: &str,
        api_key: String,
        timeout: Option<u64>,
        retry: Option<usize>,
    ) -> Result<Self, Error> {
        let mut headlines = Config::headlines(api_base_url, &api_key)?;
        let mut trusted_headlines = Config::trusted_headlines(api_base_url, &api_key)?;
        let mut news = Config::news(api_base_url, &api_key)?;
        let mut similar_news = Config::news(api_base_url, &api_key)?;
        let mut trending_topics = Config::trending_topics(api_base_url, api_key)?;
        if let Some(timeout) = timeout.map(Duration::from_millis) {
            headlines.timeout = timeout;
            trusted_headlines.timeout = timeout;
            news.timeout = timeout;
            similar_news.timeout = timeout;
            trending_topics.timeout = timeout;
        }
        if let Some(retry) = retry {
            headlines.retry = retry;
            trusted_headlines.retry = retry;
            news.retry = retry;
            similar_news.retry = retry;
            trending_topics.retry = retry;
        }

        let headlines = select_provider(
            headlines.build(),
            NewscatcherHeadlinesProvider::from_endpoint,
        )?;
        let news = select_provider(news.build(), NewscatcherNewsProvider::from_endpoint)?;
        let trusted_headlines =
            NewscatcherTrustedHeadlinesProvider::from_endpoint(trusted_headlines.build());
        let similar_news = MltSimilarNewsProvider::from_endpoint(similar_news.build());

        Ok(Providers {
            headlines,
            trusted_headlines,
            news,
            similar_news,
        })
    }
}
