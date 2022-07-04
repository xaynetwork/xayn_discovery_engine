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

use crate::document::{self, HistoricDocument};

use self::models::ApiDocumentView;

pub mod sqlite;

#[async_trait]
pub trait Storage: FeedScope {
    type StorageError;

    async fn init_database(&self) -> Result<(), <Self as Storage>::StorageError>;

    async fn fetch_history(&self)
        -> Result<Vec<HistoricDocument>, <Self as Storage>::StorageError>;

    fn feed(
        &self,
    ) -> &(dyn FeedScope<FeedScopeError = <Self as FeedScope>::FeedScopeError> + Send + Sync);
}

#[async_trait]
pub trait FeedScope {
    type FeedScopeError;

    async fn close_document(&self, document: document::Id) -> Result<(), Self::FeedScopeError>;

    async fn clear(&self) -> Result<(), Self::FeedScopeError>;

    async fn fetch(&self) -> Result<Vec<ApiDocumentView>, Self::FeedScopeError>;

    // helper function. will be replaced later by move_from_stacks_to_feed
    async fn store_documents(
        &self,
        documents: &[document::Document],
    ) -> Result<(), Self::FeedScopeError>;
}

pub mod models {

    use chrono::NaiveDateTime;
    use url::Url;
    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_providers::Market;

    use crate::document::{self, UserReaction};

    pub struct NewDocument {
        pub id: document::Id,
        pub news_resource: NewsResource,
        pub newscatcher: NewscatcherData,
        pub embedding: Embedding,
    }

    pub struct ApiDocumentView {
        pub document_id: document::Id,
        pub news_resource: NewsResource,
        pub newscatcher_data: NewscatcherData,
        pub user_reacted: Option<UserReaction>,
        // //FIXME I don't think this is helpful as multiple documents in the vec can have the same value for this!
        pub in_batch_index: u32,
    }

    /// Represents a news that is delivered by an external content API.
    #[derive(Debug, Clone)]
    pub struct NewsResource {
        /// Title of the resource.
        pub title: String,

        /// Snippet of the resource.
        pub snippet: String,

        /// Main topic of the publisher.
        pub topic: String,

        /// Url to reach the resource.
        pub url: Url,

        /// Image attached to the news.
        pub image: Option<Url>,

        /// Publishing date.
        //FIXME it's NativeDateTime in the current codebase but we can't compare
        //      NativeDateTimes across different markets, but we do! So this needs to be
        //      at least a Utc DateTime.
        pub date_published: NaiveDateTime,

        /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
        pub source: String,

        /// The market of news.
        pub market: Market,
    }

    pub struct NewscatcherData {
        pub domain_rank: u32,
        pub score: Option<f32>,
    }
}
