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

use std::sync::Arc;

use async_trait::async_trait;
use itertools::chain;
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{
    Article,
    Client,
    CommonQueryParts,
    HeadlinesQuery,
    Market,
    DEFAULT_WHEN,
};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{ArticleFilter, CommonFilter, SourcesFilter},
        Id,
    },
};

use super::{
    common::{create_requests_for_markets, request_min_new_items},
    NewItemsError,
    Ops,
};

/// Stack operations customized for breaking news items.
pub(crate) struct BreakingNews {
    client: Arc<Client>,
    markets: Arc<RwLock<Vec<Market>>>,
    excluded_sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
    max_requests: u32,
    min_articles: usize,
}

impl BreakingNews {
    /// Creates a breaking news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<Client>) -> Self {
        Self {
            client,
            markets: config.markets.clone(),
            excluded_sources: config.excluded_sources.clone(),
            page_size: config.page_size,
            max_requests: config.max_requests,
            min_articles: config.min_articles,
        }
    }

    pub(crate) fn id() -> Id {
        Id(Uuid::parse_str("1ce442c8-8a96-433e-91db-c0bee37e5a83").unwrap(/* valid uuid */))
    }

    /// Filter `articles` based on `stack` documents.
    fn filter_articles(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
        excluded_sources: &[String],
    ) -> Result<Vec<Article>, GenericError> {
        let articles = SourcesFilter::apply(articles, excluded_sources);
        CommonFilter::apply(history, stack, articles)
    }
}

#[async_trait]
impl Ops for BreakingNews {
    fn id(&self) -> Id {
        BreakingNews::id()
    }

    fn needs_key_phrases(&self) -> bool {
        false
    }

    async fn new_items(
        &self,
        _key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
    ) -> Result<Vec<Article>, NewItemsError> {
        let markets = self.markets.read().await.clone();
        let excluded_sources = Arc::new(self.excluded_sources.read().await.clone());

        request_min_new_items(
            self.max_requests,
            self.min_articles,
            |request_num| {
                create_requests_for_markets(markets.clone(), |market| {
                    let page = request_num as usize + 1;
                    spawn_headlines_request(
                        self.client.clone(),
                        market,
                        self.page_size,
                        page,
                        excluded_sources.clone(),
                    )
                })
            },
            |articles| Self::filter_articles(history, stack, articles, &excluded_sources),
        )
        .await
        .map_err(Into::into)
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        Ok(chain!(stack, new).cloned().collect())
    }
}

fn spawn_headlines_request(
    client: Arc<Client>,
    market: Market,
    page_size: usize,
    page: usize,
    excluded_sources: Arc<Vec<String>>,
) -> JoinHandle<Result<Vec<Article>, GenericError>> {
    tokio::spawn(async move {
        let market = market;
        let query = HeadlinesQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size,
                page,
                excluded_sources: &excluded_sources,
            },
            trusted_sources: &[],
            topic: None,
            when: DEFAULT_WHEN,
        };
        client.query_articles(&query).await.map_err(Into::into)
    })
}
