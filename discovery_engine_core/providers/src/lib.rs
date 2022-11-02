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

use crate::mlt::MltSimilarNewsProvider;
pub use crate::{
    config::Config,
    error::Error,
    helpers::{
        clean_query::clean_query,
        filter::{Filter, Market},
        rest_endpoint::RestEndpoint,
    },
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
    pub news: Arc<dyn NewsProvider>,
    pub similar_news: Arc<dyn SimilarNewsProvider>,
    pub headlines: Arc<dyn HeadlinesProvider>,
    pub trusted_headlines: Arc<dyn TrustedHeadlinesProvider>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_base_url: &str,
        api_key: String,
        news_provider: Option<&str>,
        similar_news_provider: Option<&str>,
        headlines_provider: Option<&str>,
        trusted_headlines_provider: Option<&str>,
        timeout: Option<u64>,
        retry: Option<usize>,
    ) -> Result<Self, Error> {
        let mut news = Config::news(api_base_url, news_provider, &api_key)?;
        let mut similar_news = Config::news(api_base_url, similar_news_provider, &api_key)?;
        let mut headlines = Config::headlines(api_base_url, headlines_provider, &api_key)?;
        let mut trusted_headlines =
            Config::trusted_headlines(api_base_url, trusted_headlines_provider, api_key)?;
        if let Some(timeout) = timeout.map(Duration::from_millis) {
            news.timeout = timeout;
            similar_news.timeout = timeout;
            headlines.timeout = timeout;
            trusted_headlines.timeout = timeout;
        }
        if let Some(retry) = retry {
            news.retry = retry;
            similar_news.retry = retry;
            headlines.retry = retry;
            trusted_headlines.retry = retry;
        }

        let news = select_provider(news.build(), NewscatcherNewsProvider::from_endpoint)?;
        let similar_news = MltSimilarNewsProvider::from_endpoint(similar_news.build());
        let headlines = select_provider(
            headlines.build(),
            NewscatcherHeadlinesProvider::from_endpoint,
        )?;
        let trusted_headlines =
            NewscatcherTrustedHeadlinesProvider::from_endpoint(trusted_headlines.build());

        Ok(Providers {
            news,
            similar_news,
            headlines,
            trusted_headlines,
        })
    }
}
