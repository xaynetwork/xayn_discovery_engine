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
use chrono::{DateTime, Utc};
use derive_more::{Deref, DerefMut, From};
use serde::{Deserialize, Serialize};
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{PositiveCoi, UserInterests};

use crate::{
    app::SetupError,
    models::{
        self,
        DocumentId,
        DocumentPropertyId,
        DocumentTag,
        IngestedDocument,
        InteractedDocument,
        PersonalizedDocument,
        UserId,
        UserInteractionType,
    },
    Error,
};

pub(crate) struct KnnSearchParams<'a, I> {
    pub(crate) excluded: I,
    pub(crate) embedding: &'a NormalizedEmbedding,
    pub(crate) k_neighbors: usize,
    // must be >= k_neighbors
    pub(crate) num_candidates: usize,
    pub(crate) published_after: Option<DateTime<Utc>>,
    pub(crate) min_similarity: Option<f32>,
    pub(crate) time: DateTime<Utc>,
}

#[derive(Debug, Deref, DerefMut, From)]
pub(crate) struct Warning<T>(Vec<T>);

impl<T> Default for Warning<T> {
    fn default() -> Self {
        Vec::default().into()
    }
}

impl<T> IntoIterator for Warning<T> {
    type Item = <Vec<T> as IntoIterator>::Item;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> FromIterator<T> for Warning<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Vec::from_iter(iter).into()
    }
}

#[async_trait(?Send)]
pub(crate) trait Document {
    async fn get_interacted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<InteractedDocument>, Error>;

    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<PersonalizedDocument>, Error>;

    async fn get_embedding(&self, id: &DocumentId) -> Result<Option<NormalizedEmbedding>, Error>;

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
    ) -> Result<Vec<PersonalizedDocument>, Error>;

    /// Inserts the documents and reports failed ids.
    async fn insert(&self, documents: Vec<IngestedDocument>) -> Result<Warning<DocumentId>, Error>;

    /// Deletes the documents and reports failed ids.
    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error>;
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
    ) -> Result<Option<Option<()>>, Error>;
}

#[async_trait]
pub(crate) trait Interest {
    async fn get(&self, user_id: &UserId) -> Result<UserInterests, Error>;
}

pub(crate) struct InteractionUpdateContext<'s, 'l> {
    pub(crate) document: &'s InteractedDocument,
    pub(crate) interaction_type: UserInteractionType,
    pub(crate) tag_weight_diff: &'s mut HashMap<&'l DocumentTag, i32>,
    pub(crate) positive_cois: &'s mut Vec<PositiveCoi>,
    pub(crate) time: DateTime<Utc>,
}

#[async_trait(?Send)]
pub(crate) trait Interaction {
    async fn get(&self, user_id: &UserId) -> Result<Vec<DocumentId>, Error>;

    async fn user_seen(&self, id: &UserId, time: DateTime<Utc>) -> Result<(), Error>;

    async fn update_interactions(
        &self,
        user_id: &UserId,
        interactions: impl IntoIterator<
            IntoIter = impl Clone + ExactSizeIterator<Item = &(DocumentId, UserInteractionType)>,
        >,
        store_user_history: bool,
        time: DateTime<Utc>,
        update_logic: impl for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> PositiveCoi,
    ) -> Result<(), Error>;
}

pub(crate) type TagWeights = HashMap<DocumentTag, usize>;

#[async_trait]
pub(crate) trait Tag {
    async fn get(&self, user_id: &UserId) -> Result<TagWeights, Error>;
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    elastic: elastic::Config,
    postgres: postgres::Config,
}

impl Config {
    pub(crate) async fn setup(&self) -> Result<Storage, SetupError> {
        let elastic = self.elastic.setup_client()?;
        let postgres = self.postgres.setup_database().await?;

        Ok(Storage { elastic, postgres })
    }
}

pub struct Storage {
    elastic: elastic::Client,
    postgres: postgres::Database,
}

impl Storage {
    pub async fn close(&self) {
        self.postgres.close().await;
    }
}
