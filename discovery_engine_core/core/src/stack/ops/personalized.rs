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
use xayn_discovery_engine_providers::{
    Article,
    Client,
    CommonQueryParts,
    Filter,
    Market,
    NewsQuery,
};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{
            dedup_documents,
            filter_semantically,
            ArticleFilter,
            CommonFilter,
            SemanticFilterConfig,
        },
        Id,
    },
};

use super::Ops;

/// Stack operations customized for personalized news items.
pub(crate) struct PersonalizedNews {
    client: Arc<Client>,
    markets: Arc<RwLock<Vec<Market>>>,
    page_size: usize,
    semantic_filter_config: SemanticFilterConfig,
}

impl PersonalizedNews {
    /// Creates a personalized news stack.
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
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("311dc7eb-5fc7-4aa4-8232-e119f7e80e76").unwrap(/* valid uuid */))
    }

    fn needs_key_phrases(&self) -> bool {
        true
    }

    async fn new_items(&self, key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        if key_phrases.is_empty() {
            return Ok(vec![]);
        }

        let mut articles = Vec::new();
        let mut errors = Vec::new();
        let filter = Arc::new(key_phrases.iter().fold(Filter::default(), |filter, kp| {
            filter.add_keyword(kp.words())
        }));

        let mut requests = self
            .markets
            .read()
            .await
            .iter()
            .cloned()
            .map(|market| {
                spawn_news_request(self.client.clone(), market, filter.clone(), self.page_size)
            })
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
        dedup_documents(&mut merged);
        let filtered = filter_semantically(merged, &self.semantic_filter_config);

        Ok(filtered)
    }
}

fn spawn_news_request(
    client: Arc<Client>,
    market: Market,
    filter: Arc<Filter>,
    page_size: usize,
) -> JoinHandle<Result<Vec<Article>, xayn_discovery_engine_providers::Error>> {
    tokio::spawn(async move {
        let market = market;
        let query = NewsQuery {
            common: CommonQueryParts {
                market: &market,
                page_size,
                page: 1,
            },
            filter,
        };

        client.query_articles(&query).await
    })
}
