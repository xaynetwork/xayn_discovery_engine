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

use std::{convert::identity, hash::Hash, ops::AddAssign};

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
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentQuery,
        DocumentSnippet,
        DocumentTags,
    },
    storage::{property_filter::IndexedPropertyType, KnnSearchParams, Warning},
    Error,
};

impl Client {
    pub(super) async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        match params.strategy {
            SearchStrategy::Knn => self.knn_search(params).await,
            SearchStrategy::Hybrid { query } => {
                let normalize_knn = identity;
                let normalize_bm25 = normalize_scores_if_max_gt_1;
                let merge_fn = merge_scores_average_duplicates_only;
                self.hybrid_search(params, query, normalize_knn, normalize_bm25, merge_fn)
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
    ) -> Result<ScoreMap<DocumentId>, Error> {
        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter: _,
        } = params.create_common_knn_search_parts();

        let request = merge_json_objects([knn_object, generic_parameters]);
        let scores = self.search_request(request).await?;

        Ok(rescale_knn_scores(scores))
    }

    async fn hybrid_search(
        &self,
        params: KnnSearchParams<'_>,
        query: &DocumentQuery,
        normalize_knn: impl FnOnce(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
        normalize_bm25: impl FnOnce(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
        merge_function: impl FnOnce(ScoreMap<DocumentId>, ScoreMap<DocumentId>) -> ScoreMap<DocumentId>,
    ) -> Result<ScoreMap<DocumentId>, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = params.create_common_knn_search_parts();

        let knn_request = merge_json_objects([knn_object, generic_parameters.clone()]);
        // don't rescale the knn_scores since they would need to be immediately normalized again to be fed into normalize_knn()
        let knn_scores = self.search_request(knn_request).await?;

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
        let bm25_scores = self.search_request(bm_25).await?;

        let merged = merge_function(normalize_knn(knn_scores), normalize_bm25(bm25_scores));
        Ok(take_highest_n_scores(count, merged))
    }

    pub(super) async fn insert_documents(
        &self,
        documents: impl IntoIterator<
            IntoIter = impl ExactSizeIterator<Item = &models::IngestedDocument>,
        >,
    ) -> Result<Warning<DocumentId>, Error> {
        let documents = documents.into_iter();
        if documents.len() == 0 {
            return Ok(Warning::default());
        }

        let response = self
            .bulk_request(documents.flat_map(|document| {
                [
                    serde_json::to_value(BulkInstruction::Index { id: &document.id }),
                    serde_json::to_value(IngestedDocument {
                        snippet: &document.snippet,
                        properties: &document.properties,
                        embedding: &document.embedding,
                        tags: &document.tags,
                    }),
                ]
            }))
            .await?;
        Ok(response.failed_documents("index", false).into())
    }

    pub(super) async fn delete_documents(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Warning<DocumentId>, Error> {
        let ids = ids.into_iter();
        if ids.len() == 0 {
            return Ok(Warning::default());
        }

        let response = self
            .bulk_request(ids.map(|id| Ok(BulkInstruction::Delete { id })))
            .await?;
        Ok(response.failed_documents("delete", true).into())
    }

    pub(super) async fn retain_documents(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-delete-by-query.html
        let url = self.create_url(["_delete_by_query"], [("refresh", None)]);
        let body = json!({
            "query": {
                "bool": {
                    "must_not": {
                        "ids": {
                            "values": ids.into_iter().collect_vec()
                        }
                    }
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
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", document_id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": update_script,
            "_source": false
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
struct IngestedDocument<'a> {
    snippet: &'a DocumentSnippet,
    properties: &'a DocumentProperties,
    embedding: &'a NormalizedEmbedding,
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

fn normalize_scores<K>(mut scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default();

    if max_score != 0. {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    scores
}

fn normalize_scores_if_max_gt_1<K>(mut scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default()
        .max(1.0);

    for score in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

fn merge_scores_average_duplicates_only<K>(
    mut scores_1: ScoreMap<K>,
    scores_2: ScoreMap<K>,
) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    for (key, value) in scores_2 {
        scores_1
            .entry(key)
            .and_modify(|score| *score = (*score + value) / 2.)
            .or_insert(value);
    }
    scores_1
}

fn merge_scores_weighted<K>(scores: impl IntoIterator<Item = (f32, ScoreMap<K>)>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let weighted = scores.into_iter().flat_map(|(weight, mut scores)| {
        for score in scores.values_mut() {
            *score *= weight;
        }
        scores
    });
    collect_summing_repeated(weighted)
}

/// Reciprocal Rank Fusion
fn rrf<K>(k: f32, scores: impl IntoIterator<Item = (f32, ScoreMap<K>)>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let rrf_scores = scores.into_iter().flat_map(|(weight, scores)| {
        scores
            .into_iter()
            .sorted_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse())
            .enumerate()
            .map(move |(rank0, (document, _))| {
                #[allow(clippy::cast_precision_loss)]
                (document, (k + rank0 as f32 + 1.).recip() * weight)
            })
    });
    collect_summing_repeated(rrf_scores)
}

fn collect_summing_repeated<K>(scores: impl IntoIterator<Item = (K, f32)>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    scores
        .into_iter()
        .fold(ScoreMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().add_assign(value);
            acc
        })
}

fn take_highest_n_scores<K>(n: usize, scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    if scores.len() <= n {
        return scores;
    }

    scores
        .into_iter()
        .sorted_unstable_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse())
        .take(n)
        .collect()
}

impl NormalizationFn {
    fn to_fn(self) -> Box<dyn Fn(ScoreMap<DocumentId>) -> ScoreMap<DocumentId>> {
        match self {
            NormalizationFn::Identity => Box::new(identity),
            NormalizationFn::Normalize => Box::new(normalize_scores),
            NormalizationFn::NormalizeIfMaxGt1 => Box::new(normalize_scores_if_max_gt_1),
        }
    }
}

type DynMergeFn = dyn Fn(ScoreMap<DocumentId>, ScoreMap<DocumentId>) -> ScoreMap<DocumentId>;

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
                let rank_constant = rank_constant.unwrap_or(60.);
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
    fn test_rrf_parameters_are_used() {
        let id = |id: &str| id.try_into().unwrap();
        let left: ScoreMap<DocumentId> = [(id("foo"), 2.), (id("bar"), 1.), (id("baz"), 3.)].into();
        let right: ScoreMap<DocumentId> = [(id("baz"), 5.), (id("dodo"), 1.2)].into();
        assert_eq!(
            rrf(80., [(1., left.clone()), (1., right.clone())]),
            [
                (id("foo"), 1. / (80. + 2.)),
                (id("bar"), 1. / (80. + 3.)),
                (id("baz"), 1. / (80. + 1.) + 1. / (80. + 1.)),
                (id("dodo"), 1. / (80. + 2.)),
            ]
            .into(),
        );
        assert_eq!(
            rrf(80., [(0.2, left), (8., right)]),
            [
                (id("foo"), 0.2 / (80. + 2.)),
                (id("bar"), 0.2 / (80. + 3.)),
                (id("baz"), 0.2 / (80. + 1.) + 8. / (80. + 1.)),
                (id("dodo"), 8. / (80. + 2.)),
            ]
            .into(),
        );
    }

    #[test]
    fn test_validate_default_index_update_config() {
        IndexUpdateConfig::default().validate().unwrap();
    }
}
