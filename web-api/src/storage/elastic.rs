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

use std::collections::HashMap;

pub(crate) use client::{Client, ClientBuilder, Config, Error as ElasticError};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use xayn_ai_bert::NormalizedEmbedding;

use self::client::BulkInstruction;
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

#[derive(Debug, Deserialize)]
/// Deserializes from any map/struct dropping all fields.
///
/// This will not work with non self describing non schema
/// formats like bincode.
struct IgnoredResponse {/* Note: The {} is needed for it to work correctly. */}

impl Client {
    #[allow(clippy::too_many_lines)]
    pub(super) async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        let time = params.time.to_rfc3339();
        // the existing documents are not filtered in the query to avoid too much work for a cold
        // path, filtering them afterwards can occasionally lead to less than k results though
        let excluded_ids = json!({
            "ids": {
                "values": params.excluded.into_iter().collect_vec()
            }
        });
        let Value::Object(mut filter) = (
            if let Some(published_after) = params.published_after {
                // published_after != null && published_after <= publication_date <= time
                json!({
                    "filter": {
                        "range": {
                            "properties.publication_date": {
                                "gte": published_after.to_rfc3339(),
                                "lte": time
                            }
                        }
                    },
                    "must_not": excluded_ids
                })
            } else {
                // published_after == null || published_after <= time
                json!({
                    "must_not": [
                        excluded_ids,
                        {
                            "range": {
                                "properties.publication_date": {
                                    "gt": time
                                }
                            }
                        }
                    ]
                })
            }
        ) else {
            unreachable!(/* filter is a json object */);
        };

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/knn-search.html#approximate-knn
        let Value::Object(mut body) = json!({
            "knn": {
                "field": "embedding",
                "query_vector": params.embedding,
                "k": params.count,
                "num_candidates": params.num_candidates,
                "filter": {
                    "bool": filter
                }
            },
            "size": params.count,
            "_source": false
        }) else {
            unreachable!(/* body is a json object */);
        };
        if let Some(min_similarity) = params.min_similarity {
            body.insert("min_score".to_string(), json!(min_similarity));
        }
        if let Some(query) = params.query {
            filter.insert("must".to_string(), json!({ "match": { "snippet": query }}));
            body.insert("query".to_string(), json!({ "bool": filter }));
            body.insert("explain".to_string(), json!(true));
        }

        let scores = self
            .query_with_json::<_, SearchResponse<NoSource>>(
                self.create_resource_path(["_search"], None),
                Some(body),
            )
            .await?
            .map(|response| {
                if params.query.is_some() {
                    // mixed knn and bm25 scores need to be normalized
                    const KNN_DESCRIPTION: &str = "within top k documents";
                    let mut max_bm25_score = None;
                    let scores = response
                        .hits
                        .hits
                        .into_iter()
                        .map(|hit| {
                            // details has exactly one or two elements: knn or bm25 or both
                            let mut knn_score = None;
                            let mut bm25_score = None;
                            let detail = &hit.explanation.details[0];
                            if detail.description == KNN_DESCRIPTION {
                                knn_score = Some(detail.value);
                            } else {
                                bm25_score = Some(detail.value);
                            }
                            if let Some(detail) = hit.explanation.details.get(1) {
                                if detail.description == KNN_DESCRIPTION {
                                    knn_score = Some(detail.value);
                                } else {
                                    bm25_score = Some(detail.value);
                                }
                            }
                            if bm25_score > max_bm25_score {
                                max_bm25_score = bm25_score;
                            }
                            (hit.id, knn_score, bm25_score)
                        })
                        .collect_vec();

                    let max_bm25_score = max_bm25_score.unwrap_or_default().max(1.0);
                    scores
                        .into_iter()
                        .map(|(id, knn_score, bm25_score)| {
                            let score = match (knn_score, bm25_score) {
                                (Some(knn_score), Some(bm25_score)) => {
                                    0.5 * knn_score + 0.5 * bm25_score / max_bm25_score
                                }
                                (Some(knn_score), None) => knn_score,
                                (None, Some(bm25_score)) => bm25_score / max_bm25_score,
                                (None, None) => unreachable!(),
                            };
                            (id, score)
                        })
                        .collect::<HashMap<_, _>>()
                } else {
                    // only knn scores are already normalized
                    response
                        .hits
                        .hits
                        .into_iter()
                        .map(|hit| (hit.id, hit.score))
                        .collect::<HashMap<_, _>>()
                }
            })
            .unwrap_or_default();

        Ok(scores)
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
        let url = self.create_resource_path(["_delete_by_query"], [("refresh", None)]);
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
        let url = self.create_resource_path(["_update", id.as_ref()], None);
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
        let url = self.create_resource_path(["_update", id.as_ref()], None);
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
        let url = self.create_resource_path(["_update", document_id.as_ref()], None);
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
        let url = self.create_resource_path(["_update", document_id.as_ref()], None);
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
        let url = self.create_resource_path(["_update", id.as_ref()], None);
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

#[derive(Debug, Deserialize)]
struct Detail {
    description: String,
    value: f32,
}

#[derive(Debug, Default, Deserialize)]
struct Explanation {
    details: Vec<Detail>,
}

#[derive(Debug, Deserialize)]
struct Hit<T> {
    #[serde(rename = "_id")]
    id: DocumentId,
    #[allow(dead_code)]
    #[serde(rename = "_source")]
    source: T,
    #[serde(rename = "_score")]
    score: f32,
    #[serde(default)]
    #[serde(rename = "_explanation")]
    explanation: Explanation,
}

#[derive(Debug, Deserialize)]
struct Hits<T> {
    hits: Vec<Hit<T>>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse<T> {
    hits: Hits<T>,
}

#[derive(Debug)]
struct NoSource;

impl<'de> Deserialize<'de> for NoSource {
    fn deserialize<D>(_: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self)
    }
}

#[derive(Debug, Serialize)]
struct IngestedDocument<'a> {
    snippet: &'a str,
    properties: &'a DocumentProperties,
    embedding: &'a NormalizedEmbedding,
    tags: &'a [DocumentTag],
}
