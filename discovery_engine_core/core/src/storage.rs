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

pub mod models;
pub mod sqlite;
mod utils;

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use displaydoc::Display;
use thiserror::Error;
use xayn_discovery_engine_ai::{GenericError, MalformedBytesEmbedding};

use crate::{
    document::{self, HistoricDocument, NewsResource, UserReaction, ViewMode, WeightedSource},
    stack,
    storage::models::{ApiDocumentView, NewDocument, Search, TimeSpentDocumentView},
};
use xayn_discovery_engine_ai::Embedding;

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

impl From<MalformedBytesEmbedding> for Error {
    fn from(err: MalformedBytesEmbedding) -> Self {
        Error::Database(Box::new(err))
    }
}

/// Hint about what was done during db init.
pub enum InitDbHint {
    /// Hint to use if nothing special happened during init.
    NormalInit,
    /// A new db was created; There was no db beforehand.
    NewDbCreated,
    /// There was a db but it could not be opened so it was deleted and a new one created.
    DbOverwrittenDueToErrors(Error),
}

#[derive(Clone, Debug)]
pub struct DartMigrationData {
    pub engine_state: Option<Vec<u8>>,
    pub trusted_sources: Vec<String>,
    pub excluded_sources: Vec<String>,
    pub reacted_sources: Vec<WeightedSource>,
    pub documents: Vec<MigrationDocument>,
    pub search: Option<Search>,
}

/// Represents a result from a query.
#[derive(Clone, Debug)]
pub struct MigrationDocument {
    /// Unique identifier of the document.
    pub id: document::Id,

    /// Stack from which the document has been taken.
    /// [`stack::Id::nil()`] is used for documents which are not from a stack
    pub stack_id: stack::Id,

    /// Embedding from smbert.
    pub smbert_embedding: Option<Embedding>,

    /// Reaction.
    pub reaction: UserReaction,

    /// Resource this document refers to.
    pub resource: NewsResource,

    // If true the document is part of the search OR feed
    pub is_active: bool,

    // If true the document is/was part of the search
    pub is_searched: bool,

    // The index of the batch in which it was returned from the engine.
    pub batch_index: u32,

    // The time at which it was returned from the engine.
    pub timestamp: DateTime<Utc>,

    pub story_view_time: Option<Duration>,
    pub web_view_time: Option<Duration>,
    pub reader_view_time: Option<Duration>,
}

#[async_trait]
pub(crate) trait Storage {
    async fn clear_database(&self) -> Result<bool, Error>;

    async fn fetch_history(&self) -> Result<Vec<HistoricDocument>, Error>;

    async fn fetch_weighted_sources(&self) -> Result<Vec<WeightedSource>, Error>;

    fn feed(&self) -> &(dyn FeedScope + Send + Sync);

    fn search(&self) -> &(dyn SearchScope + Send + Sync);

    fn feedback(&self) -> &(dyn FeedbackScope + Send + Sync);

    // temporary helper functions
    fn state(&self) -> &(dyn StateScope + Send + Sync);

    fn source_preference(&self) -> &(dyn SourcePreferenceScope + Send + Sync);

    fn source_reaction(&self) -> &(dyn SourceReactionScope + Send + Sync);
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

    async fn update_time_spent(
        &self,
        document: document::Id,
        view_mode: ViewMode,
        view_time: Duration,
    ) -> Result<TimeSpentDocumentView, Error>;

    async fn update_source_reaction(&self, source: &str, like: bool) -> Result<(), Error>;
}

#[async_trait]
pub(crate) trait StateScope {
    async fn store(&self, bytes: &[u8]) -> Result<(), Error>;

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

#[async_trait]
pub(crate) trait SourceReactionScope {
    /// Fetch the weight of a source.
    ///
    /// The weight defaults to 0 if no weight is stored.
    async fn fetch_source_weight(&self, source: &str) -> Result<i32, Error>;

    /// Updates the source weight.
    ///
    /// If no source weight was stored a new weight equals to `add_weight` will be stored.
    ///
    /// If the sign of the source weight stored differs from the sign of `add_weight` it
    /// will be set to 0 no matter the amount changed.
    ///
    /// Currently negative weight is capped at -1.
    async fn update_source_weight(&self, source: &str, add_weight: i32) -> Result<(), Error>;

    async fn delete_source_reaction(&self, source: &str) -> Result<(), Error>;
}
