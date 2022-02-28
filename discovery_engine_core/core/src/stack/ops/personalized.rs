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
use tokio::sync::RwLock;
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{Article, Client, Filter, Market, NewsQuery};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{ArticleFilter, CommonFilter},
        Id,
    },
};

use super::Ops;

/// Stack operations customized for personalized news items.
#[derive(Default)]
pub(crate) struct PersonalizedNews {
    client: Client,
    markets: Option<Arc<RwLock<Vec<Market>>>>,
    page_size: usize,
}

#[async_trait]
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("311dc7eb-5fc7-4aa4-8232-e119f7e80e76").unwrap(/* valid uuid */))
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.client = Client::new(config.api_key.clone(), config.api_base_url.clone());
        self.markets.replace(Arc::clone(&config.markets));
        self.page_size = config.page_size;
    }

    async fn new_items(&self, key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        if key_phrases.is_empty() {
            return Ok(vec![]);
        }
        if let Some(markets) = self.markets.as_ref() {
            let mut articles = Vec::new();
            let mut errors = Vec::new();
            let filter = &key_phrases.iter().fold(Filter::default(), |filter, kp| {
                filter.add_keyword(kp.words())
            });

            for market in markets.read().await.iter() {
                let query = NewsQuery {
                    market,
                    filter,
                    page_size: self.page_size,
                    page: None,
                };
                match self.client.news(&query).await {
                    Ok(batch) => articles.extend(batch),
                    Err(err) => errors.push(err),
                }
            }
            if articles.is_empty() && !errors.is_empty() {
                Err(errors.pop().unwrap(/* nonempty errors */).into())
            } else {
                Ok(articles)
            }
        } else {
            Ok(vec![])
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
        let mut res: Vec<_> = stack.into();
        res.extend_from_slice(new);
        Ok(res)
    }
}
