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

use chrono::{DateTime, Utc};
pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::elastic::{BulkInstruction, Config};

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
    utils::json_object,
    Error,
};

type JsonObject = serde_json::Map<String, Value>;

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
        // knn search with `k`/`num_candidates` set to zero is a bad request
        if params.count == 0 {
            return Ok(HashMap::new());
        }

        let mut filter =
            self.create_es_search_filter(params.excluded, params.published_after);

        let mut body = self.create_knn_request_object(
            params.embedding,
            params.count,
            params.num_candidates,
            &filter,
        );

        body.extend(json_object!({
            "size": params.count,
            "_source": false,
        }));

        if let Some(min_similarity) = params.min_similarity {
            body.extend(json_object!({
                "min_score": min_similarity,
            }));
        }

        let mut knn_scores = self.search_request(&body).await?;

        if let Some(query) = params.query {
            body.remove("knn");
            filter.extend(json_object!({
                "must": { "match": { "snippet": query }}
            }));
            body.extend(json_object!({
                "query": { "bool": filter }
            }));

            let bm25_scores = self.search_request(body).await?;
            let max_bm25_score = bm25_scores
                .values()
                .max_by(|s1, s2| s1.total_cmp(s2))
                .copied()
                .unwrap_or_default()
                .max(1.0);

            // mixed knn and bm25 scores need to be normalized
            for (id, bm25_score) in bm25_scores {
                let bm25_score = bm25_score / max_bm25_score;
                knn_scores
                    .entry(id)
                    .and_modify(|knn_score| *knn_score = 0.5 * *knn_score + 0.5 * bm25_score)
                    .or_insert(bm25_score);
            }

            knn_scores = knn_scores
                .into_iter()
                .sorted_unstable_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse())
                .take(params.count)
                .collect();
        }

        Ok(knn_scores)
    }

    async fn search_knn(&self) {
        todo!();
    }

    async fn search_hybrid(&self) {
        todo!()
    }

    fn create_es_search_filter<'a>(
        &self,
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
        &self,
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
