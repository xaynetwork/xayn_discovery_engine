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
use xayn_discovery_engine_providers::{Article, Client, HeadlinesQuery, Market};

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError},
    stack::Id,
};

use super::Ops;

/// Stack operations customized for breaking news items.
#[derive(Default)]
pub(crate) struct BreakingNews {
    client: Client,
    markets: Option<Arc<RwLock<Vec<Market>>>>,
    page_size: Option<usize>,
}

#[async_trait]
impl Ops for BreakingNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("1ce442c8-8a96-433e-91db-c0bee37e5a83").unwrap(/* valid uuid */))
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.client = Client::new(config.api_key.clone(), config.api_base_url.clone());
        self.markets.replace(Arc::clone(&config.markets));
        self.page_size.replace(config.api_page_size);
    }

    async fn new_items(&self, _key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        if let Some(markets) = self.markets.as_ref() {
            let mut articles = Vec::new();
            let mut errors = Vec::new();
            let page_size = Some(20); // TODO pass through config later

            for market in markets.read().await.clone() {
                let query = HeadlinesQuery { market, page_size };
                match self.client.headlines(&query).await {
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
