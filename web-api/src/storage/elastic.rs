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

use std::{collections::HashMap, convert::identity, hash::Hash, ops::AddAssign};

pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::elastic::{BulkInstruction, Config};
use xayn_web_api_shared::serde::{json_object, merge_json_objects, JsonObject};

use super::{MergeFn, NormalizationFn, SearchStrategy};
use crate::{
    models::{
        self,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentTag,
    },
    storage::{KnnSearchParams, Warning},
    Error,
};

/// Deserializes from any map/struct dropping all fields.
///
/// This will not work with non self describing non schema
/// formats like bincode.
#[derive(Debug, Deserialize)]
struct IgnoredResponse {/* Note: The {} is needed for it to work correctly. */}

impl Client {
    pub(super) async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        match params.strategy {
            SearchStrategy::Knn => self.knn_search(params).await,
            SearchStrategy::Hybrid { query } => {
                let normalize_knn = identity;
                let normalize_bm25 = normalize_scores_if_max_gt_1;
                let merge_fn = merge_scores_average_duplicates_only;
                self.hybrid_search(params, query, normalize_knn, normalize_bm25, merge_fn)
                    .await
            }
            SearchStrategy::DevHybrid {
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
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter: _,
        } = params.create_common_knn_search_parts();

        let request = merge_json_objects([knn_object, generic_parameters]);

        Ok(self.search_request(request).await?)
    }

    async fn hybrid_search<'a>(
        &self,
        params: KnnSearchParams<'a>,
        query: &'a str,
        normalize_knn: impl FnOnce(ScoreMap) -> ScoreMap,
        normalize_bm25: impl FnOnce(ScoreMap) -> ScoreMap,
        merge_function: impl FnOnce(ScoreMap, ScoreMap) -> ScoreMap,
    ) -> Result<ScoreMap, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = params.create_common_knn_search_parts();

        let knn_request = merge_json_objects([knn_object, generic_parameters.clone()]);

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

    async fn hybrid_search_es_rrf<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
        query: &'a str,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        let count = params.count;

        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        } = create_common_knn_search_parts(params);

        let request = merge_json_objects([
            knn_object,
            generic_parameters,
            json_object!({
                "query": { "bool": merge_json_objects([
                    inner_filter,
                    json_object!({
                        "must": { "match": { "snippet": query }}
                    })
                ]) },
                "rank": {
                    "rrf": {
                        // must be >= "size"
                        "rank_constant": count
                    }
                }
            }),
        ]);

        // TODO[pmk/now] should we normalize
        Ok(self.search_request(request).await?)
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
                    serde_json::to_value(BulkInstruction::Index { id: &document.id })
                        .map_err(Into::into),
                    serde_json::to_value(IngestedDocument {
                        snippet: &document.snippet,
                        properties: &document.properties,
                        embedding: &document.embedding,
                        tags: &document.tags,
                    })
                    .map_err(Into::into),
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
        self.query_with_json::<_, IgnoredResponse>(url, Some(body))
            .await?;

        Ok(())
    }

    pub(super) async fn insert_document_properties(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": properties
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    pub(super) async fn delete_document_properties(
        &self,
        id: &DocumentId,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": DocumentProperties::new()
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    pub(super) async fn insert_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", document_id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties.put(params.prop_id, params.property)",
                "params": {
                    "prop_id": property_id,
                    "property": property
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    pub(super) async fn delete_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<()>>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", document_id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties.remove(params.prop_id)",
                "params": {
                    "prop_id": property_id
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| Some(())))
    }

    pub(super) async fn insert_document_tags(
        &self,
        id: &DocumentId,
        tags: &[DocumentTag],
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", id.as_ref()], [("refresh", None)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.tags = params.tags",
                "params": {
                    "tags": tags
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }
}

#[derive(Debug, Serialize)]
struct IngestedDocument<'a> {
    snippet: &'a str,
    properties: &'a DocumentProperties,
    embedding: &'a NormalizedEmbedding,
    tags: &'a [DocumentTag],
}

struct KnnSearchParts {
    knn_object: JsonObject,
    generic_parameters: JsonObject,
    inner_filter: JsonObject,
}

impl KnnSearchParams<'_> {
    fn create_common_knn_search_parts(&self) -> KnnSearchParts {
        let inner_filter = self.create_search_filter();
        let knn_object = self.create_knn_request_object(&inner_filter);

        let mut generic_parameters = json_object!({
            "size": self.count
        });

        if let Some(min_score) = self.min_similarity {
            generic_parameters.extend(json_object!({
                "min_score": min_score,
            }));
        }

        KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter,
        }
    }

    fn create_search_filter(&self) -> JsonObject {
        let mut filter = JsonObject::new();
        if !self.excluded.is_empty() {
            // existing documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            filter.insert(
                "must_not".to_string(),
                json!({ "ids": { "values": self.excluded } }),
            );
        }
        if let Some(published_after) = self.published_after {
            // published_after != null && published_after <= publication_date
            let published_after = published_after.to_rfc3339();
            filter.insert(
                "filter".to_string(),
                json!({ "range": { "properties.publication_date": { "gte": published_after } } }),
            );
        }
        filter
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

fn normalize_scores<K>(mut scores: HashMap<K, f32>) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default();

    for score in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

fn normalize_scores_if_max_gt_1<K>(mut scores: HashMap<K, f32>) -> HashMap<K, f32>
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
    mut scores_1: HashMap<K, f32>,
    scores_2: HashMap<K, f32>,
) -> HashMap<K, f32>
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

fn merge_scores_weighted<K>(
    scores: impl IntoIterator<Item = (f32, HashMap<K, f32>)>,
) -> HashMap<K, f32>
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
fn rrf<K>(k: f32, scores: impl IntoIterator<Item = HashMap<K, f32>>) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    let rrf_scores = scores
        .into_iter()
        .map(|scores| {
            scores
                .into_iter()
                .sorted_by(|(_, s1), (_, s2)| s1.total_cmp(&s2).reverse())
                .enumerate()
                .map(|(rank0, (document, _))| (document, (k + rank0 as f32).recip()))
        })
        .flatten();
    collect_summing_repeated(rrf_scores)
}

fn collect_summing_repeated<K>(scores: impl IntoIterator<Item = (K, f32)>) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    scores
        .into_iter()
        .fold(HashMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().add_assign(value);
            acc
        })
}

fn take_highest_n_scores<K>(n: usize, scores: HashMap<K, f32>) -> HashMap<K, f32>
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

type ScoreMap = HashMap<DocumentId, f32>;

impl NormalizationFn {
    fn to_fn(self) -> Box<dyn Fn(ScoreMap) -> ScoreMap> {
        match self {
            NormalizationFn::Identity => Box::new(identity),
            NormalizationFn::Normalize => Box::new(normalize_scores),
            NormalizationFn::NormalizeIfMaxGt1 => Box::new(normalize_scores_if_max_gt_1),
        }
    }
}

impl MergeFn {
    fn to_fn(self) -> Box<dyn Fn(ScoreMap, ScoreMap) -> ScoreMap> {
        match self {
            MergeFn::Weighted => Box::new(|l, r| merge_scores_weighted([(0.5, l), (0.5, r)])),
            MergeFn::AverageDuplicatesOnly => Box::new(merge_scores_average_duplicates_only),
        }
    }
}
