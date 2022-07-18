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

use async_trait::async_trait;
use displaydoc::Display;
use thiserror::Error;
use xayn_discovery_engine_ai::GenericError;

use crate::document::{self, HistoricDocument};

use self::models::{ApiDocumentView, NewDocument};

pub mod sqlite;

pub(crate) type BoxedStorage = Box<dyn Storage + Send + Sync>;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Database error: {0}
    Database(#[source] GenericError),
}

#[async_trait]
pub(crate) trait Storage {
    async fn init_database(&self) -> Result<(), Error>;

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, Error>;

    fn feed(&self) -> &(dyn FeedScope + Send + Sync);
}

#[async_trait]
pub(crate) trait FeedScope {
    async fn close_document(&self, document: &document::Id) -> Result<(), Error>;

    async fn clear(&self) -> Result<(), Error>;

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Error>;

    // helper function. will be replaced later by move_from_stacks_to_feed
    async fn store_documents(&self, documents: &[NewDocument]) -> Result<(), Error>;
}

pub mod models {
    use chrono::NaiveDateTime;
    use url::Url;
    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_providers::Market;

    use crate::document::{self, UserReaction};

    pub(crate) struct NewDocument {
        pub(crate) id: document::Id,
        pub(crate) news_resource: NewsResource,
        pub(crate) newscatcher_data: NewscatcherData,
        #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub(crate) struct ApiDocumentView {
        pub(crate) document_id: document::Id,
        pub(crate) news_resource: NewsResource,
        pub(crate) newscatcher_data: NewscatcherData,
        pub(crate) user_reacted: Option<UserReaction>,
        // //FIXME I don't think this is helpful as multiple documents in the vec can have the same value for this!
        pub(crate) in_batch_index: u32,
    }

    /// Represents a news that is delivered by an external content API.
    #[derive(Debug, Clone)]
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

    pub(crate) struct NewscatcherData {
        pub(crate) domain_rank: u64,
        pub(crate) score: Option<f32>,
    }
}
