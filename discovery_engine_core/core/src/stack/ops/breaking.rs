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

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError, Market},
    ranker::Ranker,
    stack::Id,
};

use super::Ops;

/// Stack operations customized for breaking news items.
// NOTE mock implementation for now
struct BreakingNews {
    markets: Option<Arc<RwLock<Vec<Market>>>>,
}

#[async_trait]
impl Ops for BreakingNews {
    fn id(&self) -> Id {
        todo!();
    }

    fn configure(&mut self, config: EndpointConfig) {
        let _prev_mkts = self.markets.replace(config.markets);
    }

    async fn new_items<'a>(
        &self,
        _key_phrases: &[KeyPhrase],
        _ranker: &'a (dyn Ranker + Sync),
    ) -> Result<Vec<Document>, GenericError> {
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
