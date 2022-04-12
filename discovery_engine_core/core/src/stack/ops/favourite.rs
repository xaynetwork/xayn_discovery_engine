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
use xayn_discovery_engine_providers::{Article, Client, CommonQueryParts, HeadlinesQuery};

use crate::{
    document::{dedup_documents, Document, HistoricDocument},
    engine::{EndpointConfig, GenericError},
    stack::{
        filters::{filter_semantically, ArticleFilter, CommonFilter, SemanticFilterConfig},
        Id,
    },
};

use super::{common::request_min_new_items, Ops};

/// Stack operations customized for favourite news.
pub(crate) struct FavouriteNews {
    client: Arc<Client>,
    sources: Arc<RwLock<Vec<String>>>,
    page_size: usize,
    semantic_filter_config: SemanticFilterConfig,
    max_requests: u32,
    min_articles: usize,
}

impl FavouriteNews {
    /// Creates a favourite news stack.
    pub(crate) fn new(config: &EndpointConfig, client: Arc<Client>) -> Self {
        Self {
            client,
            sources: config.favourite_sources.clone(),
            page_size: config.page_size,
            semantic_filter_config: SemanticFilterConfig::default(),
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
impl Ops for FavouriteNews {
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
    ) -> Result<Vec<Article>, GenericError> {
        let sources = Arc::new(self.sources.read().await.clone());
        request_min_new_items(
            self.max_requests,
            self.min_articles,
            |request_num| {
                let page = request_num as usize + 1;
                let future = spawn_favourites_request(
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
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        let mut merged = chain!(stack, new).cloned().collect();
        dedup_documents(&mut merged);
        let filtered = filter_semantically(merged, &self.semantic_filter_config);

        Ok(filtered)
    }
}

fn spawn_favourites_request(
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
            topic: None,
        };
        client.query_articles(&query).await.map_err(Into::into)
    })
}
