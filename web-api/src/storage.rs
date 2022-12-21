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

pub(crate) mod elastic;
#[cfg(test)]
pub(crate) mod memory;
pub(crate) mod postgres;
mod utils;

use std::collections::HashMap;

use async_trait::async_trait;
use derive_more::From;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use xayn_ai_coi::{Embedding, PositiveCoi, UserInterests};

use crate::{
    error::common::DocumentIdAsObject,
    models::{
        self,
        DocumentId,
        DocumentPropertyId,
        IngestedDocument,
        PersonalizedDocument,
        UserId,
    },
    server::SetupError,
    Error,
};

pub(crate) struct KnnSearchParams<'a> {
    pub(crate) excluded: &'a [DocumentId],
    pub(crate) embedding: &'a Embedding,
    pub(crate) k_neighbors: usize,
    // must be >= k_neighbors
    pub(crate) num_candidates: usize,
}

#[derive(Debug, Error, From)]
pub(crate) enum InsertionError {
    #[error("{0}")]
    General(Error),
    #[error("{failed_documents:?}")]
    PartialFailure {
        failed_documents: Vec<DocumentIdAsObject>,
    },
}

#[derive(Debug, From)]
pub(crate) enum DeletionError {
    General(Error),
    PartialFailure { errors: Vec<DocumentIdAsObject> },
}

#[async_trait]
pub(crate) trait Document {
    async fn get_by_ids(&self, ids: &[&DocumentId]) -> Result<Vec<PersonalizedDocument>, Error>;

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<PersonalizedDocument>, Error>;

    async fn insert(
        &self,
        documents: Vec<(IngestedDocument, Embedding)>,
    ) -> Result<(), InsertionError>;

    async fn delete(&self, documents: &[DocumentId]) -> Result<(), DeletionError>;
}

#[async_trait]
pub(crate) trait DocumentProperties {
    async fn get(&self, id: &DocumentId) -> Result<Option<models::DocumentProperties>, Error>;

    async fn put(
        &self,
        id: &DocumentId,
        properties: &models::DocumentProperties,
    ) -> Result<Option<()>, Error>;

    async fn delete(&self, id: &DocumentId) -> Result<Option<()>, Error>;
}

#[async_trait]
pub(crate) trait DocumentProperty {
    async fn get(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<models::DocumentProperty>>, Error>;

    async fn put(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &models::DocumentProperty,
    ) -> Result<Option<()>, Error>;

    async fn delete(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error>;
}

#[async_trait]
pub(crate) trait Interest {
    async fn get(&self, user_id: &UserId) -> Result<UserInterests, Error>;

    async fn update_positive<F>(
        &self,
        doc_id: &DocumentId,
        user_id: &UserId,
        update_cois: F,
    ) -> Result<(), Error>
    where
        F: Fn(&mut Vec<PositiveCoi>) -> &PositiveCoi + Send + Sync;
}

#[async_trait]
pub(crate) trait Interaction {
    async fn get(&self, user_id: &UserId) -> Result<Vec<DocumentId>, Error>;

    async fn user_seen(&self, id: &UserId) -> Result<(), Error>;
}

#[async_trait]
pub(crate) trait Tag {
    async fn get(&self, user_id: &UserId) -> Result<HashMap<String, usize>, Error>;

    async fn update(&self, user_id: &UserId, tags: &[String]) -> Result<(), Error>;
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct Config {
    #[serde(default)]
    elastic: elastic::Config,
    #[serde(default)]
    postgres: postgres::Config,
}

impl Config {
    pub(crate) async fn setup(&self) -> Result<Storage, SetupError> {
        let elastic = self.elastic.setup_client()?;
        let postgres = self.postgres.setup_database().await?;

        Ok(Storage { elastic, postgres })
    }
}

pub(crate) struct Storage {
    elastic: elastic::Client,
    postgres: postgres::Database,
}
