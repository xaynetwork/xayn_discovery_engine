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
use chrono::NaiveDate;
use tokio::sync::RwLock;
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{Article, Client, Filter, Market, NewsQuery};

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError},
    stack::Id,
};

use super::Ops;

/// Stack operations customized for personalized news items.
// NOTE mock implementation for now
#[derive(Default)]
pub(crate) struct PersonalizedNews {
    token: String,
    url: String,
    markets: Option<Arc<RwLock<Vec<Market>>>>,
}

#[async_trait]
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("311dc7eb-5fc7-4aa4-8232-e119f7e80e76").unwrap(/* valid uuid */))
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.token.clone_from(&config.api_key);
        self.url.clone_from(&config.api_base_url);
        self.markets
            .replace(Arc::new(tokio::sync::RwLock::new(vec![]))); // FIXME
    }

    async fn new_items(&self, key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        Ok(if let Some(markets) = self.markets.as_ref() {
            let client = Client::new(self.token.clone(), self.url.clone());
            let mut articles = Vec::new();
            for market in markets.read().await.clone() {
                let page_size = None; // FIXME
                let filter = key_phrases.iter().fold(Filter::default(), |filter, kp| {
                    filter.add_keyword(kp.words())
                });
                let query = NewsQuery {
                    market,
                    filter,
                    page_size,
                };
                articles.extend(client.news(&query).await?);
            }
            articles
        } else {
            vec![]
        })
    }

    fn filter_articles(
        &self,
        _current: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        Ok(articles)
    }

    fn merge(&self, current: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        let mut res: Vec<_> = current.into();
        res.extend_from_slice(new);
        Ok(res)
    }
}
