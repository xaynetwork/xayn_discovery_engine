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
pub(crate) mod property_filter;
mod utils;

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use derive_more::{Deref, DerefMut, From};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::Coi;
use xayn_web_api_db_ctrl::{tenant::Tenant, LegacyTenantInfo, Silo};
use xayn_web_api_shared::{postgres as postgres_shared, request::TenantId};

use self::property_filter::{IndexedPropertiesSchema, IndexedPropertiesSchemaUpdate};
use crate::{
    app::SetupError,
    backoffice::IngestionConfig,
    frontoffice::filter::Filter,
    models::{
        self,
        DocumentForIngestion,
        DocumentId,
        DocumentPropertyId,
        DocumentQuery,
        DocumentTag,
        DocumentTags,
        ExcerptedDocument,
        PersonalizedDocument,
        SnippetForInteraction,
        SnippetId,
        SnippetOrDocumentId,
        UserId,
    },
    tenants,
    Error,
};

pub(crate) struct KnnSearchParams<'a> {
    pub(crate) excluded: &'a Exclusions,
    pub(crate) embedding: &'a NormalizedEmbedding,
    /// The number of documents which will be returned if there are enough fitting documents.
    pub(crate) count: usize,
    // must be >= count
    pub(crate) num_candidates: usize,
    pub(super) strategy: SearchStrategy<'a>,
    pub(super) include_properties: bool,
    pub(super) include_snippet: bool,
    pub(super) filter: Option<&'a Filter>,
    pub(super) with_raw_scores: bool,
}

#[derive(Default)]
pub(crate) struct Exclusions {
    pub(crate) documents: Vec<DocumentId>,
    pub(crate) snippets: Vec<SnippetId>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SearchStrategy<'a> {
    Knn,
    Hybrid {
        /// An additional query which will be run in parallel with the KNN search.
        query: &'a DocumentQuery,
    },
    HybridDev {
        query: &'a DocumentQuery,
        normalize_knn: NormalizationFn,
        normalize_bm25: NormalizationFn,
        merge_fn: MergeFn,
    },
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NormalizationFn {
    Identity,
    Normalize,
    NormalizeIfMaxGt1,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) enum MergeFn {
    Sum {
        #[serde(default)]
        knn_weight: Option<f32>,
        #[serde(default)]
        bm25_weight: Option<f32>,
    },
    AverageDuplicatesOnly {},
    Rrf {
        #[serde(default)]
        rank_constant: Option<f32>,
        #[serde(default)]
        knn_weight: Option<f32>,
        #[serde(default)]
        bm25_weight: Option<f32>,
    },
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
    async fn get_snippets_for_interaction(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
    ) -> Result<Vec<SnippetForInteraction>, Error>;

    //FIXME this is only used by (view) tests and dead code, consider removing it
    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &SnippetId>>,
        include_properties: bool,
        include_snippet: bool,
    ) -> Result<Vec<PersonalizedDocument>, Error>;

    async fn get_excerpted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId> + Clone>,
    ) -> Result<Vec<ExcerptedDocument>, Error>;

    async fn get_embedding(&self, id: &SnippetId) -> Result<Option<NormalizedEmbedding>, Error>;

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<PersonalizedDocument>, Error>;

    /// Inserts the documents and reports failed ids.
    async fn insert(
        &self,
        documents: Vec<DocumentForIngestion>,
    ) -> Result<Warning<DocumentId>, Error>;

    /// Deletes the documents and reports failed ids.
    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error>;
}

#[async_trait(?Send)]
pub(crate) trait DocumentCandidate {
    /// Gets the document candidates.
    async fn get(&self) -> Result<Vec<DocumentId>, Error>;

    /// Sets the document candidates and reports failed ids.
    async fn set(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error>;

    /// Adds the document candidates and reports failed ids.
    async fn add(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
    ) -> Result<Warning<DocumentId>, Error>;

    /// Removes the document candidates and reports failed ids.
    async fn remove(
        &self,
        ids: impl IntoIterator<Item = &DocumentId>,
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
    async fn get(&self, user_id: &UserId) -> Result<Vec<Coi>, Error>;
}

pub(crate) struct InteractionUpdateContext<'s, 'l> {
    pub(crate) document: &'s SnippetForInteraction,
    pub(crate) tag_weight_diff: &'s mut HashMap<&'l DocumentTag, i32>,
    pub(crate) interests: &'s mut Vec<Coi>,
    pub(crate) time: DateTime<Utc>,
}

#[async_trait(?Send)]
pub(crate) trait Interaction {
    async fn get(&self, user_id: &UserId) -> Result<Vec<DocumentId>, Error>;

    async fn user_seen(&self, id: &UserId, time: DateTime<Utc>) -> Result<(), Error>;

    async fn update_interactions(
        &self,
        user_id: &UserId,
        interactions: Vec<SnippetOrDocumentId>,
        store_user_history: bool,
        time: DateTime<Utc>,
        update_logic: impl for<'a, 'b> FnMut(InteractionUpdateContext<'a, 'b>) -> Coi,
    ) -> Result<(), Error>;
}

pub(crate) type TagWeights = HashMap<DocumentTag, usize>;

#[async_trait]
pub(crate) trait Tag {
    /// Gets the weighted tags for a user.
    async fn get(&self, user_id: &UserId) -> Result<TagWeights, Error>;

    /// Sets the document tags if the document exists.
    async fn put(&self, document_id: &DocumentId, tags: &DocumentTags)
        -> Result<Option<()>, Error>;
}

#[async_trait(?Send)]
pub(crate) trait Size {
    /// Gets the size in bytes of the json value.
    async fn json(&self, value: &Value) -> Result<usize, Error>;
}

#[async_trait(?Send)]
pub(crate) trait IndexedProperties {
    async fn load_schema(&self) -> Result<IndexedPropertiesSchema, Error>;

    async fn extend_schema(
        &self,
        update: IndexedPropertiesSchemaUpdate,
        ingestion_config: &IngestionConfig,
    ) -> Result<IndexedPropertiesSchema, Error>;
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Config {
    elastic: elastic::Config,
    postgres: postgres_shared::Config,
}

pub(crate) struct Storage {
    tenant: Tenant,
    elastic: elastic::Client,
    postgres: postgres::Database,
}

impl Storage {
    pub(crate) async fn builder(
        config: &Config,
        legacy_tenant: Option<TenantId>,
    ) -> Result<StorageBuilder, SetupError> {
        Ok(StorageBuilder {
            elastic: elastic::Client::builder(config.elastic.clone())?,
            postgres: postgres::Database::builder(&config.postgres, legacy_tenant).await?,
        })
    }

    pub(crate) fn tenant(&self) -> &Tenant {
        &self.tenant
    }
}

// FIXME: long term this should be run by the control plane,
//        in a different binary/lambda or similar before we
//        start updating the instances.
pub(crate) async fn initialize_silo(
    config: &Config,
    tenant_config: &tenants::Config,
    embedding_sizes: HashMap<String, usize>,
) -> Result<(Silo, Option<TenantId>), SetupError> {
    let silo = Silo::new(
        config.postgres.clone(),
        config.elastic.clone(),
        tenant_config
            .enable_legacy_tenant
            .then(|| LegacyTenantInfo {
                es_index: config.elastic.index_name.clone(),
            }),
        embedding_sizes,
    )
    .await?;

    // FIXME: remove this once we have a proper separation between
    //        a admin pg user owning the db structure and a web-api-mt
    //        user which can only use tables but nothing more.
    silo.admin_as_mt_user_hack().await?;

    let legacy_tenant = silo.initialize().await?;
    Ok((silo, legacy_tenant))
}

#[derive(Clone)]
pub(crate) struct StorageBuilder {
    elastic: elastic::ClientBuilder,
    postgres: postgres::DatabaseBuilder,
}

impl StorageBuilder {
    pub(crate) async fn build_for(&self, tenant_id: TenantId) -> Result<Storage, Error> {
        let tenant = {
            let mut connection = self.postgres.mt_user_connection().await?;
            Tenant::load_from_postgres(&mut connection, tenant_id).await?
        };
        let elastic = self.elastic.build_for(&tenant);
        let postgres = self.postgres.build_for(&tenant);
        Ok(Storage {
            tenant,
            elastic,
            postgres,
        })
    }

    pub(crate) async fn close(&self) {
        self.postgres.close().await;
    }

    pub(crate) fn legacy_tenant(&self) -> Option<&TenantId> {
        self.postgres.legacy_tenant()
    }
}
