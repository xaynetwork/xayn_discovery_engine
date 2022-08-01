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

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
use xayn_discovery_engine_ai::GenericError;

use crate::{
    document::{self, HistoricDocument, UserReaction},
    stack,
};

use self::models::{ApiDocumentView, NewDocument, Search};

pub mod sqlite;
mod utils;

pub(crate) type BoxedStorage = Box<dyn Storage + Send + Sync>;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Database error: {0}
    Database(#[source] GenericError),
    /// Search request failed: open search
    OpenSearch,
    /// Search request failed: no search
    NoSearch,
    /// Search request failed: no document with id {0}
    NoDocument(document::Id),
}

impl From<sqlx::Error> for Error {
    fn from(generic: sqlx::Error) -> Self {
        Error::Database(generic.into())
    }
}

#[async_trait]
pub(crate) trait Storage {
    async fn init_database(&self) -> Result<(), Error>;

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, Error>;

    fn feed(&self) -> &(dyn FeedScope + Send + Sync);

    fn search(&self) -> &(dyn SearchScope + Send + Sync);

    fn feedback(&self) -> &(dyn FeedbackScope + Send + Sync);

    // temporary helper functions
    fn state(&self) -> &(dyn StateScope + Send + Sync);

    fn source_preference(&self) -> &(dyn SourcePreferenceScope + Send + Sync);
}

#[async_trait]
pub(crate) trait FeedScope {
    async fn delete_documents(&self, ids: &[document::Id]) -> Result<bool, Error>;

    async fn clear(&self) -> Result<bool, Error>;

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Error>;

    // helper function. will be replaced later by move_from_stacks_to_feed
    async fn store_documents(
        &self,
        documents: &[NewDocument],
        stack_ids: &HashMap<document::Id, stack::Id>,
    ) -> Result<(), Error>;
}

#[async_trait]
pub(crate) trait SearchScope {
    async fn store_new_search(
        &self,
        search: &Search,
        documents: &[NewDocument],
    ) -> Result<(), Error>;

    async fn store_next_page(
        &self,
        page_number: u32,
        documents: &[NewDocument],
    ) -> Result<(), Error>;

    async fn fetch(&self) -> Result<(Search, Vec<ApiDocumentView>), Error>;

    async fn clear(&self) -> Result<bool, Error>;

    //FIXME Return a `DeepSearchTemplateView` or similar in the future which
    //      only contains the necessary fields (snippet, title, smbert_embedding, market).
    async fn get_document(&self, id: document::Id) -> Result<ApiDocumentView, Error>;
}

#[async_trait]
pub(crate) trait FeedbackScope {
    async fn update_user_reaction(
        &self,
        document: document::Id,
        reaction: UserReaction,
    ) -> Result<ApiDocumentView, Error>;
}

#[async_trait]
pub(crate) trait StateScope {
    async fn store(&self, bytes: Vec<u8>) -> Result<(), Error>;

    async fn fetch(&self) -> Result<Option<Vec<u8>>, Error>;

    async fn clear(&self) -> Result<bool, Error>;
}

#[async_trait]
pub(crate) trait SourcePreferenceScope {
    async fn set_trusted(&self, sources: &HashSet<String>) -> Result<(), Error>;

    async fn set_excluded(&self, sources: &HashSet<String>) -> Result<(), Error>;

    async fn fetch_trusted(&self) -> Result<HashSet<String>, Error>;

    async fn fetch_excluded(&self) -> Result<HashSet<String>, Error>;
}

pub mod models {
    use chrono::NaiveDateTime;
    use url::Url;
    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_providers::Market;

    use crate::{
        document::{self, UserReaction},
        stack,
    };

    #[derive(Debug)]
    pub(crate) struct NewDocument {
        pub(crate) id: document::Id,
        pub(crate) news_resource: NewsResource,
        pub(crate) newscatcher_data: NewscatcherData,
        pub(crate) embedding: Embedding,
    }

    impl From<document::Document> for NewDocument {
        fn from(doc: document::Document) -> Self {
            let (news_resource, newscatcher_data) = doc.resource.into();
            Self {
                id: doc.id,
                news_resource,
                newscatcher_data,
                embedding: doc.smbert_embedding,
            }
        }
    }

    impl From<document::NewsResource> for (NewsResource, NewscatcherData) {
        fn from(resource: document::NewsResource) -> Self {
            let news_resource = NewsResource {
                title: resource.title,
                snippet: resource.snippet,
                topic: resource.topic,
                url: resource.url,
                image: resource.image,
                date_published: resource.date_published,
                source: resource.source_domain,
                market: Market::new(resource.language, resource.country),
            };
            let newscatcher_data = NewscatcherData {
                domain_rank: resource.rank,
                score: resource.score,
            };
            (news_resource, newscatcher_data)
        }
    }

    impl From<(NewsResource, NewscatcherData)> for document::NewsResource {
        fn from((news_resource, newscatcher_data): (NewsResource, NewscatcherData)) -> Self {
            Self {
                title: news_resource.title,
                snippet: news_resource.snippet,
                url: news_resource.url,
                source_domain: news_resource.source,
                date_published: news_resource.date_published,
                image: news_resource.image,
                rank: newscatcher_data.domain_rank,
                score: newscatcher_data.score,
                country: news_resource.market.country_code,
                language: news_resource.market.lang_code,
                topic: news_resource.topic,
            }
        }
    }

    #[derive(Debug)]
    pub(crate) struct ApiDocumentView {
        pub(crate) document_id: document::Id,
        pub(crate) news_resource: NewsResource,
        pub(crate) newscatcher_data: NewscatcherData,
        pub(crate) user_reacted: Option<UserReaction>,
        pub(crate) embedding: Embedding,
        pub(crate) stack_id: Option<stack::Id>,
    }

    impl ApiDocumentView {
        /// Gets the snippet or falls back to the title if the snippet is empty.
        pub(crate) fn snippet_or_title(&self) -> &str {
            (!self.news_resource.snippet.is_empty())
                .then(|| &self.news_resource.snippet)
                .unwrap_or(&self.news_resource.title)
        }
    }

    impl From<ApiDocumentView> for document::Document {
        fn from(view: ApiDocumentView) -> Self {
            document::Document {
                id: view.document_id,
                stack_id: view.stack_id.unwrap_or_default(),
                smbert_embedding: view.embedding,
                resource: (view.news_resource, view.newscatcher_data).into(),
            }
        }
    }

    /// Represents a news that is delivered by an external content API.
    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct NewsResource {
        /// Title of the resource.
        pub(crate) title: String,

        /// Snippet of the resource.
        pub(crate) snippet: String,

        /// Main topic of the publisher.
        pub(crate) topic: String,

        /// Url to reach the resource.
        pub(crate) url: Url,

        /// Image attached to the news.
        pub(crate) image: Option<Url>,

        /// Publishing date.
        // FIXME: it's NativeDateTime in the current codebase but we can't compare
        //      NativeDateTimes across different markets, but we do! So this needs to be
        //      at least a Utc DateTime.
        pub(crate) date_published: NaiveDateTime,

        /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
        pub(crate) source: String,

        /// The market of news.
        pub(crate) market: Market,
    }

    #[derive(Debug, PartialEq)]
    pub(crate) struct NewscatcherData {
        pub(crate) domain_rank: u64,
        pub(crate) score: Option<f32>,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct Search {
        pub(crate) search_by: SearchBy,
        pub(crate) search_term: String,
        pub(crate) paging: Paging,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy, num_derive::FromPrimitive)]
    pub(crate) enum SearchBy {
        Query = 0,
        Topic = 1,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(crate) struct Paging {
        pub(crate) size: u32,
        pub(crate) next_page: u32,
    }
}
