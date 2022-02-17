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
        todo!();
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.markets.replace(Arc::clone(&config.markets));
    }

    async fn new_items(&self, _key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        todo!();
    }

    fn filter_articles(
        &self,
        _current: &[Document],
        _articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        todo!();
    }

    fn merge(
        &self,
        _current: &[Document],
        _new: &[Document],
    ) -> Result<Vec<Document>, GenericError> {
        todo!();
    }
}
