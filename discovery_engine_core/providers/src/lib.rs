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

use crate::mlt::MltSimilarSearchProvider;
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
        Rank,
        RankLimit,
        SearchQuery,
        SimilarSearchQuery,
        TrustedHeadlinesQuery,
        UrlWithDomain,
    },
    newscatcher::{
        Article as NewscatcherArticle,
        NewscatcherHeadlinesProvider,
        NewscatcherSearchProvider,
        NewscatcherTrustedHeadlinesProvider,
        Response as NewscatcherResponse,
    },
};

/// Provider for search.
#[async_trait]
pub trait SearchProvider: Send + Sync {
    async fn query_search(&self, query: &SearchQuery<'_>) -> Result<Vec<GenericArticle>, Error>;
}

/// Provider for similar search.
#[async_trait]
pub trait SimilarSearchProvider: Send + Sync {
    async fn query_similar_search(
        &self,
        query: &SimilarSearchQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

/// Provider for headlines.
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
    async fn query_trusted_headlines(
        &self,
        query: &TrustedHeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error>;
}

pub struct Providers {
    pub search: Arc<dyn SearchProvider>,
    pub similar_search: Arc<dyn SimilarSearchProvider>,
    pub headlines: Arc<dyn HeadlinesProvider>,
    pub trusted_headlines: Arc<dyn TrustedHeadlinesProvider>,
}

impl Providers {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_base_url: &str,
        api_key: String,
        search_provider: Option<&str>,
        similar_search_provider: Option<&str>,
        headlines_provider: Option<&str>,
        trusted_headlines_provider: Option<&str>,
        timeout: Option<u64>,
        retry: Option<usize>,
    ) -> Result<Self, Error> {
        let mut search = Config::search(api_base_url, search_provider, &api_key)?;
        let mut similar_search =
            Config::similar_search(api_base_url, similar_search_provider, &api_key)?;
        let mut headlines = Config::headlines(api_base_url, headlines_provider, &api_key)?;
        let mut trusted_headlines =
            Config::trusted_headlines(api_base_url, trusted_headlines_provider, api_key)?;
        if let Some(timeout) = timeout.map(Duration::from_millis) {
            search.timeout = timeout;
            similar_search.timeout = timeout;
            headlines.timeout = timeout;
            trusted_headlines.timeout = timeout;
        }
        if let Some(retry) = retry {
            search.retry = retry;
            similar_search.retry = retry;
            headlines.retry = retry;
            trusted_headlines.retry = retry;
        }

        let search = NewscatcherSearchProvider::from_endpoint(search.build());
        let similar_search = MltSimilarSearchProvider::from_endpoint(similar_search.build());
        let headlines = NewscatcherHeadlinesProvider::from_endpoint(headlines.build());
        let trusted_headlines =
            NewscatcherTrustedHeadlinesProvider::from_endpoint(trusted_headlines.build());

        Ok(Providers {
            search,
            similar_search,
            headlines,
            trusted_headlines,
        })
    }
}
