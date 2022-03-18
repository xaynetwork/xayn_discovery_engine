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
use futures::{stream::FuturesUnordered, StreamExt};
use itertools::chain;
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{Article, Client, CommonQueryParts, HeadlinesQuery, Market};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{filter_semantically, ArticleFilter, CommonFilter, SemanticFilterConfig},
        Id,
    },
};

use super::Ops;

/// Stack operations customized for breaking news items.
pub(crate) struct BreakingNews {
    client: Arc<Client>,
    markets: Arc<RwLock<Vec<Market>>>,
    page_size: usize,
    semantic_filter_config: SemanticFilterConfig,
}

impl BreakingNews {
    /// Creates a breaking news stack.
    pub(crate) fn new(config: &EndpointConfig) -> Self {
        Self {
            client: Arc::new(Client::new(&config.api_key, &config.api_base_url)),
            markets: config.markets.clone(),
            page_size: config.page_size,
            semantic_filter_config: SemanticFilterConfig::default(),
        }
    }
}

#[async_trait]
impl Ops for BreakingNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("1ce442c8-8a96-433e-91db-c0bee37e5a83").unwrap(/* valid uuid */))
    }

    fn needs_key_phrases(&self) -> bool {
        false
    }

    async fn new_items(&self, _key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        let mut articles = Vec::new();
        let mut errors = Vec::new();

        let mut requests = self
            .markets
            .read()
            .await
            .iter()
            .cloned()
            .map(|market| spawn_headlines_request(self.client.clone(), market, self.page_size))
            .collect::<FuturesUnordered<_>>();

        while let Some(handle) = requests.next().await {
            // should we also push handle errors?
            if let Ok(result) = handle {
                match result {
                    Ok(batch) => articles.extend(batch),
                    Err(err) => errors.push(err),
                }
            }
        }

        if articles.is_empty() && !errors.is_empty() {
            Err(errors.pop().unwrap(/* nonempty errors */).into())
        } else {
            Ok(articles)
        }
    }

    fn filter_articles(
        &self,
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        CommonFilter::apply(history, stack, articles)
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        let merged = chain!(stack, new).cloned().collect();
        let filtered = filter_semantically(merged, &self.semantic_filter_config);

        Ok(filtered)
    }
}

fn spawn_headlines_request(
    client: Arc<Client>,
    market: Market,
    page_size: usize,
) -> JoinHandle<Result<Vec<Article>, xayn_discovery_engine_providers::Error>> {
    tokio::spawn(async move {
        let market = market;
        let query = HeadlinesQuery {
            common: CommonQueryParts {
                market: &market,
                page_size,
                page: 1,
            },
        };
        client.query_articles(&query).await
    })
}
