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
    default_from,
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
        filters::{ArticleFilter, CommonFilter},
        Id,
    },
};

use super::{
    common::{create_requests_for_markets, request_min_new_items},
    NewItemsError,
    Ops,
};

/// Stack operations customized for personalized news items.
pub(crate) struct PersonalizedNews {
    client: Arc<Client>,
    markets: Arc<RwLock<Vec<Market>>>,
    excluded_sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
    max_requests: u32,
    min_articles: usize,
}

impl PersonalizedNews {
    /// Creates a personalized news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<Client>) -> Self {
        Self {
            client,
            markets: config.markets.clone(),
            page_size: config.page_size,
            excluded_sources: config.excluded_sources.clone(),
            max_requests: config.max_requests,
            min_articles: config.min_articles,
        }
    }

    /// Filter `articles` based on `stack` documents.
    fn filter_articles(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        CommonFilter::apply(history, stack, articles)
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

    async fn new_items(
        &self,
        key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
    ) -> Result<Vec<Article>, NewItemsError> {
        if key_phrases.is_empty() {
            return Err(NewItemsError::NotReady);
        }

        let filter = Arc::new(key_phrases.iter().fold(Filter::default(), |filter, kp| {
            filter.add_keyword(kp.words())
        }));
        let markets = self.markets.read().await.clone();
        let excluded_sources = Arc::new(self.excluded_sources.read().await.clone());

        request_min_new_items(
            self.max_requests,
            self.min_articles,
            |request_num| {
                create_requests_for_markets(markets.clone(), |market| {
                    let page = request_num as usize + 1;
                    spawn_news_request(
                        self.client.clone(),
                        market,
                        filter.clone(),
                        self.page_size,
                        page,
                        excluded_sources.clone(),
                    )
                })
            },
            |articles| Self::filter_articles(history, stack, articles),
        )
        .await
        .map_err(Into::into)
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        Ok(chain!(stack, new).cloned().collect())
    }
}

fn spawn_news_request(
    client: Arc<Client>,
    market: Market,
    filter: Arc<Filter>,
    page_size: usize,
    page: usize,
    excluded_sources: Arc<Vec<String>>,
) -> JoinHandle<Result<Vec<Article>, GenericError>> {
    tokio::spawn(async move {
        let market = market;
        let query = NewsQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size,
                page,
                excluded_sources: &excluded_sources,
            },
            filter,
            from: default_from().into(),
        };
        client.query_articles(&query).await.map_err(Into::into)
    })
}
