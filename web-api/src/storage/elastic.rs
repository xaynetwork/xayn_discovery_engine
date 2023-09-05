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

mod client;
mod filter;

use std::{collections::HashSet, convert::identity};

use anyhow::bail;
pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::elastic::{BulkInstruction, Config};
use xayn_web_api_shared::{
    elastic::{NotFoundAsOptionExt, ScoreMap, SerdeDiscard},
    serde::{json_object, merge_json_objects, JsonObject},
};

use self::filter::Clauses;
use super::{
    property_filter::IndexedPropertiesSchemaUpdate,
    MergeFn,
    NormalizationFn,
    SearchStrategy,
};
use crate::{
    app::SetupError,
    models::{
        self,
        DocumentContent,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentQuery,
        DocumentSnippet,
        DocumentTags,
        SnippetId,
    },
    rank_merge::{
        merge_scores_average_duplicates_only,
        merge_scores_weighted,
        normalize_scores,
        normalize_scores_if_max_gt_1,
        rrf,
        take_highest_n_scores,
        DEFAULT_RRF_K,
    },
    storage::{property_filter::IndexedPropertyType, KnnSearchParams, Warning},
    Error,
};

impl Client {
    pub(super) async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<ScoreMap<SnippetId>, Error> {
        match params.strategy {
            SearchStrategy::Knn => self.knn_search(params).await,
            SearchStrategy::Hybrid { query } => {
                let merge_fn = |knn, bm25| rrf(DEFAULT_RRF_K, [(1.0, knn), (1.0, bm25)]);
                self.hybrid_search(params, query, identity, identity, merge_fn)
                    .await
            }
            SearchStrategy::HybridDev {
                query,
                normalize_knn,
                normalize_bm25,
                merge_fn,
            } => {
                self.hybrid_search(
                    params,
                    query,
                    normalize_knn.to_fn(),
                    normalize_bm25.to_fn(),
                    merge_fn.to_fn(),
                )
                .await
            }
        }
    }

    async fn knn_search<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<ScoreMap<SnippetId>, Error> {
        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter: _,
        } = params.create_common_knn_search_parts();

        let request = merge_json_objects([knn_object, generic_parameters]);
        let scores = self
            .search_request(request, SnippetId::try_from_es_id)
            .await?;

        Ok(rescale_knn_scores(scores))
    }

    async fn hybrid_search(
        &self,
        params: KnnSearchParams<'_>,
        query: &DocumentQuery,
        normalize_knn: impl FnOnce(ScoreMap<SnippetId>) -> ScoreMap<SnippetId>,
        normalize_bm25: impl FnOnce(ScoreMap<SnippetId>) -> ScoreMap<SnippetId>,
        merge_function: impl FnOnce(ScoreMap<SnippetId>, ScoreMap<SnippetId>) -> ScoreMap<SnippetId>,
    ) -> Result<ScoreMap<SnippetId>, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = params.create_common_knn_search_parts();

        let knn_request = merge_json_objects([knn_object, generic_parameters.clone()]);
        // don't rescale the knn_scores since they would need to be immediately normalized again to be fed into normalize_knn()
        let knn_scores = self
            .search_request(knn_request, SnippetId::try_from_es_id)
            .await?;

        let bm_25 = merge_json_objects([
            json_object!({
                "query": { "bool": merge_json_objects([
                    inner_filter,
                    json_object!({
                        "must": { "match": { "snippet": query }}
                    })
                ]) }
            }),
            generic_parameters,
        ]);
        // FIXME parallelize polling
        let bm25_scores = self
            .search_request(bm_25, SnippetId::try_from_es_id)
            .await?;

        let merged = merge_function(normalize_knn(knn_scores), normalize_bm25(bm25_scores));
        Ok(take_highest_n_scores(count, merged))
    }

    pub(super) async fn upsert_documents(
        &self,
        documents: &[models::DocumentForIngestion],
    ) -> Result<Warning<DocumentId>, Error> {
        let ids = documents.iter().map(|document| &document.id).collect_vec();
        self.delete_by_parents(ids).await?;
        self.freshly_insert_documents(documents).await
    }

    pub(super) async fn freshly_insert_documents(
        &self,
        documents: impl IntoIterator<Item = &models::DocumentForIngestion>,
    ) -> Result<Warning<DocumentId>, Error> {
        let mut snippets = documents
            .into_iter()
            .flat_map(|document| {
                document.snippets.iter().enumerate().flat_map(
                    |(idx, DocumentContent { snippet, embedding })| {
                        #[allow(clippy::cast_possible_truncation)]
                        let id = SnippetId::new(document.id.clone(), idx as _);
                        let header =
                            serde_json::to_value(BulkInstruction::Create { id: id.to_es_id() });
                        let data = serde_json::to_value(Document {
                            snippet,
                            properties: &document.properties,
                            embedding,
                            tags: &document.tags,
                            parent: id.document_id(),
                        });

                        [header, data]
                    },
                )
            })
            .peekable();

        if snippets.peek().is_none() {
            return Ok(Warning::default());
        }

        let response = self.bulk_request(snippets).await?;
        Ok(response.failed_documents("index", false).into())
    }

    pub(super) async fn delete_by_parents(
        &self,
        parents: impl SerializeDocumentIds,
    ) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-delete-by-query.html
        let url = self.create_url(["_delete_by_query"], [("refresh", None)]);
        let body = json!({
            "query": {
                "terms": {
                    "parent": parents,
                }
            }
        });
        self.query_with_json::<_, SerdeDiscard>(Method::POST, url, Some(body))
            .await?;
        Ok(())
    }

    pub(super) async fn insert_document_properties(
        &self,
        document_id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        self.document_update(
            document_id,
            json_object!({
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": properties
                }
            }),
        )
        .await
    }

    pub(super) async fn delete_document_properties(
        &self,
        document_id: &DocumentId,
    ) -> Result<Option<()>, Error> {
        self.document_update(
            document_id,
            json_object!({
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": DocumentProperties::default()
                }
            }),
        )
        .await
    }

    pub(super) async fn insert_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        self.document_update(
            document_id,
            json_object!({
                "source": "ctx._source.properties.put(params.prop_id, params.property)",
                "params": {
                    "prop_id": property_id,
                    "property": property
                }
            }),
        )
        .await
    }

    pub(super) async fn delete_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error> {
        self.document_update(
            document_id,
            json_object!({
                "source": "ctx._source.properties.remove(params.prop_id)",
                "params": {
                    "prop_id": property_id
                }
            }),
        )
        .await
    }

    pub(super) async fn insert_document_tags(
        &self,
        document_id: &DocumentId,
        tags: &DocumentTags,
    ) -> Result<Option<()>, Error> {
        self.document_update(
            document_id,
            json_object!({
                "source": "ctx._source.tags = params.tags",
                "params": {
                    "tags": tags
                }
            }),
        )
        .await
    }

    async fn document_update(
        &self,
        document_id: &DocumentId,
        update_script: JsonObject,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update-by-query.html
        let url = self.create_url(["_update_by_query"], [("refresh", None)]);
        let body = Some(json!({
            "query": {
                "term": {
                    "parent": document_id,
                }
            },
            "script": update_script,
        }));

        Ok(self
            .query_with_json::<_, SerdeDiscard>(Method::POST, url, body)
            .await
            .not_found_as_option()?
            .map(|_| ()))
    }

    pub(super) async fn extend_mapping(
        &self,
        updates: &IndexedPropertiesSchemaUpdate,
        index_update_config: &IndexUpdateConfig,
    ) -> Result<(), Error> {
        if updates.len() == 0 {
            return Ok(());
        }
        let mut properties = JsonObject::with_capacity(updates.len());
        for (id, definition) in updates {
            // We ignore malformed values here for two reasons:
            // 1. set/add candidates can push documents to ES which contain
            //    malformed values
            // 2. if we somehow end up with a out-of-sync schema at least everything
            //    else but this property will still work correctly
            //
            // Note that `keyword` is excluded as it accepts anything and in turn is
            // from ES POV never malformed.
            let def = match definition.r#type {
                IndexedPropertyType::Boolean => {
                    json!({ "type": "boolean", "ignore_malformed": true })
                }
                IndexedPropertyType::Number => {
                    json!({ "type": "double", "ignore_malformed": true })
                }
                IndexedPropertyType::Keyword | IndexedPropertyType::KeywordArray => {
                    json!({ "type": "keyword" })
                }
                IndexedPropertyType::Date => json!({ "type": "date", "ignore_malformed": true }),
            };
            properties.insert(id.as_str().into(), def);
        }

        let body = json!({
            "properties": {
                "properties": {
                    // the properties of the field named properties in the properties of a document
                    "properties": properties
                }
            }
        });

        let url = self.create_url(["_mapping"], []);
        self.query_with_json::<_, SerdeDiscard>(Method::PUT, url, Some(body))
            .await?;

        info!("extended ES _mapping");

        self.update_indices(index_update_config).await?;

        Ok(())
    }

    async fn update_indices(&self, config: &IndexUpdateConfig) -> Result<(), Error> {
        let wait_for_completion = match config.method {
            IndexUpdateMethod::Background => false,
            IndexUpdateMethod::DangerWaitForCompletion => true,
        };

        let url = self.create_url(
            ["_update_by_query"],
            [
                ("conflicts", Some("proceed")),
                //FIXME add a way to async query the status of the update
                //      ES will return a task handle we can use for this.
                (
                    "wait_for_completion",
                    Some(&wait_for_completion.to_string()),
                ),
                ("refresh", Some("true")),
                (
                    "requests_per_second",
                    Some(&config.requests_per_second.to_string()),
                ),
            ],
        );
        self.query_with_json::<(), SerdeDiscard>(Method::POST, url, None)
            .await?;

        info!("started index update process");

        Ok(())
    }
}

pub(super) trait SerializeDocumentIds: Serialize {}
impl<T> SerializeDocumentIds for &'_ T where T: SerializeDocumentIds + ?Sized {}
impl SerializeDocumentIds for [DocumentId] {}
impl SerializeDocumentIds for [&'_ DocumentId] {}
impl SerializeDocumentIds for Vec<DocumentId> {}
impl SerializeDocumentIds for Vec<&'_ DocumentId> {}
impl SerializeDocumentIds for HashSet<DocumentId> {}
impl SerializeDocumentIds for HashSet<&'_ DocumentId> {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct IndexUpdateConfig {
    requests_per_second: usize,
    method: IndexUpdateMethod,
}

impl Default for IndexUpdateConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 500,
            method: IndexUpdateMethod::Background,
        }
    }
}

impl IndexUpdateConfig {
    pub(crate) fn validate(&self) -> Result<(), SetupError> {
        if self.requests_per_second == 0 {
            bail!("invalid IndexUpdateConfig, requests_per_second must be > 0");
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IndexUpdateMethod {
    /// Run updates in the background without feedback for completion.
    Background,
    /// This can put the DB into an inconsistent state if it hits timeouts.
    ///
    /// Never use this in production.
    DangerWaitForCompletion,
}

#[derive(Debug, Serialize)]
struct Document<'a> {
    snippet: &'a DocumentSnippet,
    properties: &'a DocumentProperties,
    embedding: &'a NormalizedEmbedding,
    parent: &'a DocumentId,
    tags: &'a DocumentTags,
}

struct KnnSearchParts {
    knn_object: JsonObject,
    generic_parameters: JsonObject,
    inner_filter: JsonObject,
}

impl KnnSearchParams<'_> {
    fn create_common_knn_search_parts(&self) -> KnnSearchParts {
        let Ok(Value::Object(inner_filter)) =
            serde_json::to_value(Clauses::new(self.filter, self.excluded))
        else {
            unreachable!(/* filter clauses is valid json object */);
        };
        let knn_object = self.create_knn_request_object(&inner_filter);
        let generic_parameters = json_object!({ "size": self.count });

        KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        }
    }

    fn create_knn_request_object(&self, filter: &JsonObject) -> JsonObject {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
        let mut obj = json_object!({
            "knn": {
                "field": "embedding",
                "query_vector": self.embedding,
                "k": self.count,
                "num_candidates": self.num_candidates,
            }
        });
        if !filter.is_empty() {
            obj["knn"]
                .as_object_mut()
                .unwrap()
                .insert("filter".into(), json!({ "bool": filter }));
        }
        obj
    }
}

// https://www.elastic.co/guide/en/elasticsearch/reference/current/dense-vector.html#dense-vector-similarity
fn rescale_knn_scores<K>(mut scores: ScoreMap<K>) -> ScoreMap<K> {
    for score in scores.values_mut() {
        *score = *score * 2. - 1.;
    }

    scores
}

impl NormalizationFn {
    fn to_fn(self) -> Box<dyn Fn(ScoreMap<SnippetId>) -> ScoreMap<SnippetId>> {
        match self {
            NormalizationFn::Identity => Box::new(identity),
            NormalizationFn::Normalize => Box::new(normalize_scores),
            NormalizationFn::NormalizeIfMaxGt1 => Box::new(normalize_scores_if_max_gt_1),
        }
    }
}

type DynMergeFn = dyn Fn(ScoreMap<SnippetId>, ScoreMap<SnippetId>) -> ScoreMap<SnippetId>;

impl MergeFn {
    fn to_fn(self) -> Box<DynMergeFn> {
        match self {
            MergeFn::Sum {
                knn_weight,
                bm25_weight,
            } => Box::new(move |knn, bm25| {
                merge_scores_weighted([
                    (knn_weight.unwrap_or(0.5), knn),
                    (bm25_weight.unwrap_or(0.5), bm25),
                ])
            }),
            MergeFn::AverageDuplicatesOnly {} => Box::new(merge_scores_average_duplicates_only),
            MergeFn::Rrf {
                rank_constant,
                knn_weight,
                bm25_weight,
            } => {
                let rank_constant = rank_constant.unwrap_or(DEFAULT_RRF_K);
                let knn_weight = knn_weight.unwrap_or(1.);
                let bm25_weight = bm25_weight.unwrap_or(1.);
                Box::new(move |knn, bm25| {
                    rrf(rank_constant, [(knn_weight, knn), (bm25_weight, bm25)])
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_index_update_config() {
        IndexUpdateConfig::default().validate().unwrap();
    }
}
