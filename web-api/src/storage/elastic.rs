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

use std::{collections::HashMap, hash::Hash};

use chrono::{DateTime, Utc};
pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::json;
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::elastic::{BulkInstruction, Config};
use xayn_web_api_shared::{
    json_object,
    serde::{merge_json_objects, JsonObject},
};

use super::SearchStrategy;
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
        params: KnnSearchParams<
            'a,
            impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &'a DocumentId>>,
        >,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        match params.strategy {
            SearchStrategy::Knn => self.knn_search(params).await,
            SearchStrategy::HybridWeighted { query } => {
                self.hybrid_search_weighted(params, query).await
            }
        }
    }

    async fn knn_search<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        let KnnSearchParts {
            knn_object,
            generic_parameters,
            inner_filter: _,
        } = create_common_knn_search_parts(params);

        let request = merge_json_objects([knn_object, generic_parameters]);

        // TODO[pmk/now] is it correct to not normalize this
        Ok(self.search_request(request).await?)
    }

    async fn hybrid_search_weighted<'a>(
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

        let knn_request = merge_json_objects([knn_object, generic_parameters.clone()]);

        // TODO[pmk/now] the code originally didn't normalize the KNN request, that seems wrong
        //      and will likely strongly discount it
        let knn_scores = normalize_scores(self.search_request(knn_request).await?);

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

        // TODO[pmk/now] fixme parallelize polling
        let bm25_scores = normalize_scores(self.search_request(bm_25).await?);
        let merged = merge_scores_weighted([(0.5, knn_scores), (0.5, bm25_scores)]);
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

fn create_common_knn_search_parts<'a>(
    params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
) -> KnnSearchParts {
    let inner_filter =
        create_es_search_filter(params.time, params.excluded, params.published_after);

    let knn_object = create_knn_request_object(
        params.embedding,
        params.count,
        params.num_candidates,
        &inner_filter,
    );

    let mut generic_parameters = json_object!({
        "size": params.count
    });

    if let Some(min_score) = params.min_similarity {
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

fn create_es_search_filter<'a>(
    excluded_ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &'a DocumentId>>,
    published_after: Option<DateTime<Utc>>,
) -> JsonObject {
    let mut filter = JsonObject::new();
    let excluded = excluded_ids.into_iter();
    if excluded.len() > 0 {
        // existing documents are not filtered in the query to avoid too much work for a cold
        // path, filtering them afterwards can occasionally lead to less than k results though
        filter.insert(
            "must_not".to_string(),
            json!({ "ids": { "values": excluded.collect_vec() } }),
        );
    }
    if let Some(published_after) = published_after {
        // published_after != null && published_after <= publication_date
        let published_after = published_after.to_rfc3339();
        filter.insert(
            "filter".to_string(),
            json!({ "range": { "properties.publication_date": { "gte": published_after } } }),
        );
    }
    filter
}

fn create_knn_request_object(
    embedding: &NormalizedEmbedding,
    count: usize,
    num_candidates: usize,
    filter: &JsonObject,
) -> JsonObject {
    // https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
    json_object!({
        "knn": {
            "field": "embedding",
            "query_vector": embedding,
            "k": count,
            "num_candidates": num_candidates,
            "filter": {
                "bool": filter
            }
        }
    })
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
    // TODO[pmk/now] the original code did a .max(1.0); this looked like bug, verify

    for score in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

fn merge_scores_weighted<K>(
    scores: impl IntoIterator<Item = (f32, HashMap<K, f32>)>,
) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    // TODO[pmk/now] this originally didn't apply weights to scores where there was only one hit
    //               this seemed like a bug and was removed, verify if that is correct
    scores
        .into_iter()
        .map(|(weight, mut scores)| {
            for score in scores.values_mut() {
                *score *= weight;
            }
            scores
        })
        .reduce(|mut acc, scores| {
            for (key, score) in scores {
                acc.entry(key)
                    .and_modify(|acc_score| *acc_score += score)
                    .or_insert(score);
            }
            acc
        })
        .unwrap_or_default()
}

fn take_highest_n_scores<K>(n: usize, scores: HashMap<K, f32>) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    scores
        .into_iter()
        .sorted_unstable_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse())
        .take(n)
        .collect()
}
