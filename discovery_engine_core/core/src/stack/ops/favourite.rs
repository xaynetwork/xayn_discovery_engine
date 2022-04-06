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
use tokio::{sync::RwLock, task::JoinHandle};
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{Article, Client, CommonQueryParts, HeadlinesQuery};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::Id,
};

use super::Ops;

/// Stack operations customized for favourite news.
pub(crate) struct FavouriteNews {
    client: Arc<Client>,
    sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
}

impl FavouriteNews {
    /// Creates a favourite news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<Client>) -> Self {
        Self {
            client,
            sources: config.favourite_sources.clone(),
            page_size: config.page_size,
        }
    }
}

#[async_trait]
impl Ops for FavouriteNews {
    fn id(&self) -> Id {
        todo!()
    }

    fn needs_key_phrases(&self) -> bool {
        false
    }

    async fn new_items(
        &self,
        _key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
    ) -> Result<Vec<Article>, GenericError> {
        todo!()
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        todo!()
    }
}

fn spawn_headlines_request(
    client: Arc<Client>,
    page_size: usize,
    page: usize,
    sources: Arc<Vec<String>>,
) -> JoinHandle<Result<Vec<Article>, GenericError>> {
    tokio::spawn(async move {
        let query = HeadlinesQuery {
            common: CommonQueryParts {
                market: None,
                page_size,
                page,
                excluded_sources: &[],
            },
            sources: &sources,
        };
        client.query_articles(&query).await.map_err(Into::into)
    })
}
