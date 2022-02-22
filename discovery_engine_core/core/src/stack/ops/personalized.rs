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
use xayn_discovery_engine_providers::Article;

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError, Market},
    stack::Id,
};

use super::Ops;

/// Stack operations customized for personalized news items.
// NOTE mock implementation for now
#[derive(Default)]
pub(crate) struct PersonalizedNews {
    markets: Option<Arc<RwLock<Vec<Market>>>>,
}

#[async_trait]
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("7861796e-0d0a-4a82-8054-193220aa63c6").unwrap(/* valid uuid */))
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.markets.replace(Arc::clone(&config.markets));
    }

    async fn new_items(&self, _key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        Ok(vec![])
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
