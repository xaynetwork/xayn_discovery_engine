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

use std::{iter, sync::Arc};

use async_trait::async_trait;
use futures::stream::FuturesUnordered;
use itertools::chain;
use tokio::{sync::RwLock, task::JoinHandle};
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;
use xayn_discovery_engine_providers::{
    Article,
    Client,
    CommonQueryParts,
    HeadlinesQuery,
    DEFAULT_WHEN,
};

use crate::{
    document::{Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{ArticleFilter, CommonFilter},
        Id,
    },
};

use super::{common::request_min_new_items, NewItemsError, Ops};

/// Stack operations customized for trusted news.
pub(crate) struct TrustedNews {
    client: Arc<Client>,
    sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
    max_requests: u32,
    min_articles: usize,
}

impl TrustedNews {
    #[allow(unused)]
    /// Creates a trusted news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<Client>) -> Self {
        Self {
            client,
            sources: config.trusted_sources.clone(),
            page_size: config.page_size,
            max_requests: config.max_requests,
            min_articles: config.min_articles,
        }
    }

    fn filter_articles(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        CommonFilter::apply(history, stack, articles)
    }
}

#[async_trait]
impl Ops for TrustedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("d0f699d8-60d2-4008-b3a1-df1cffc4b8a3").unwrap(/* valid uuid */))
    }

    fn needs_key_phrases(&self) -> bool {
        false
    }

    async fn new_items(
        &self,
        _key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
    ) -> Result<Vec<Article>, NewItemsError> {
        let sources = Arc::new(self.sources.read().await.clone());
        if sources.is_empty() {
            return Err(NewItemsError::NotReady);
        }

        request_min_new_items(
            self.max_requests,
            self.min_articles,
            |request_num| {
                let page = request_num as usize + 1;
                let future = spawn_trusted_request(
                    self.client.clone(),
                    self.page_size,
                    page,
                    sources.clone(),
                );
                iter::once(future).collect::<FuturesUnordered<_>>()
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

fn spawn_trusted_request(
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
            trusted_sources: &sources,
            topic: None,
            when: DEFAULT_WHEN,
        };
        client.query_articles(&query).await.map_err(Into::into)
    })
}
