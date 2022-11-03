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
use uuid::uuid;
use xayn_discovery_engine_ai::{GenericError, KeyPhrase};
use xayn_discovery_engine_providers::{
    GenericArticle,
    Market,
    RankLimit,
    SimilarSearchProvider,
    SimilarSearchQuery,
};

use crate::{
    config::EndpointConfig,
    document::{Document, HistoricDocument},
    stack::{
        filters::{ArticleFilter, CommonFilter, SourcesFilter},
        Id,
    },
};

use super::{common::request_min_new_items, NewItemsError, Ops};

/// Stack operations customized for personalized news items.
pub struct PersonalizedNews {
    client: Arc<dyn SimilarSearchProvider>,
    excluded_sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
    max_requests: usize,
    min_articles: usize,
    max_article_age_days: usize,
}

impl PersonalizedNews {
    pub const fn id() -> Id {
        Id(uuid!("311dc7eb-5fc7-4aa4-8232-e119f7e80e76"))
    }

    pub const fn name() -> &'static str {
        "PersonalizedNews"
    }

    /// Creates a personalized news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<dyn SimilarSearchProvider>) -> Self {
        Self {
            client,
            page_size: config.page_size,
            excluded_sources: config.excluded_sources.clone(),
            max_requests: config.max_requests,
            min_articles: config.min_articles,
            max_article_age_days: config.max_article_age_days,
        }
    }

    /// Filter `articles` based on `stack` documents.
    fn filter_articles(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<GenericArticle>,
        excluded_sources: &[String],
    ) -> Result<Vec<GenericArticle>, GenericError> {
        let articles = SourcesFilter::apply(articles, excluded_sources);
        CommonFilter::apply(history, stack, articles)
    }
}

#[async_trait]
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Self::id()
    }

    fn needs_key_phrases(&self) -> bool {
        true
    }

    async fn new_items(
        &self,
        key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
        market: &Market,
    ) -> Result<Vec<GenericArticle>, NewItemsError> {
        if key_phrases.is_empty() {
            return Err(NewItemsError::NotReady);
        }

        let phrase = key_phrases
            .iter()
            .map(|kp| kp.words().to_string())
            .reduce(|combined, kp| format!("{} {}", combined, kp))
            .unwrap(/* nonempty key_phrases */);

        let excluded_sources = Arc::new(self.excluded_sources.read().await.clone());

        request_min_new_items(
            self.max_requests,
            self.min_articles,
            self.page_size,
            |request_num| {
                spawn_search_request(
                    self.client.clone(),
                    market.clone(),
                    phrase.to_string(),
                    self.page_size,
                    request_num + 1,
                    excluded_sources.clone(),
                    self.max_article_age_days,
                )
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

fn spawn_search_request(
    client: Arc<dyn SimilarSearchProvider>,
    market: Market,
    like: String,
    page_size: usize,
    page: usize,
    excluded_sources: Arc<Vec<String>>,
    max_article_age_days: usize,
) -> JoinHandle<Result<Vec<GenericArticle>, GenericError>> {
    tokio::spawn(async move {
        let market = market;
        let query = SimilarSearchQuery {
            market: &market,
            page_size,
            page,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &excluded_sources,
            like: &like,
            max_age_days: Some(max_article_age_days),
        };
        client
            .query_similar_search(&query)
            .await
            .map_err(Into::into)
    })
}
