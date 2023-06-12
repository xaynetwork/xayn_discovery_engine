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

pub(crate) use client::{Client, ClientBuilder};
use itertools::Itertools;
use serde::Serialize;
use serde_json::{json, Map, Value};
use xayn_ai_bert::NormalizedEmbedding;
pub(crate) use xayn_web_api_shared::{
    elastic::{BulkInstruction, Config},
    url::NO_PARAM_VALUE,
};

use crate::{
    models::{
        self,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentTag,
    },
    storage::{utils::IgnoredResponse, KnnSearchParams, Warning},
    Error,
};

impl Client {
    pub(super) async fn get_by_embedding(
        &self,
        params: KnnSearchParams<
            '_,
            impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
        >,
    ) -> Result<HashMap<DocumentId, f32>, Error> {
        // knn search with `k`/`num_candidates` set to zero is a bad request
        if params.count == 0 {
            return Ok(HashMap::new());
        }

        let mut filter = Map::new();
        let excluded = params.excluded.into_iter();
        if excluded.len() > 0 {
            // existing documents are not filtered in the query to avoid too much work for a cold
            // path, filtering them afterwards can occasionally lead to less than k results though
            filter.insert(
                "must_not".to_string(),
                json!({ "ids": { "values": excluded.collect_vec() } }),
            );
        }
        if let Some(published_after) = params.published_after {
            // published_after != null && published_after <= publication_date
            let published_after = published_after.to_rfc3339();
            filter.insert(
                "filter".to_string(),
                json!({ "range": { "properties.publication_date": { "gte": published_after } } }),
            );
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/search-search.html
        let Value::Object(mut knn) = json!({
            "field": "embedding",
            "k": params.count,
            "num_candidates": params.num_candidates,
            "query_vector": params.embedding
        }) else {
            unreachable!(/* knn is a json object */);
        };
        if !filter.is_empty() {
            knn.insert("filter".to_string(), json!({ "bool": filter }));
        }
        let Value::Object(mut body) = json!({
            "knn": knn,
            "size": params.count,
            "_source": false
        }) else {
            unreachable!(/* body is a json object */);
        };
        if let Some(min_similarity) = params.min_similarity {
            body.insert("min_score".to_string(), json!(min_similarity));
        }
        let mut knn_scores = self.search_request(&body).await?;

        if let Some(query) = params.query {
            body.remove("knn");
            filter.insert("must".to_string(), json!({ "match": { "snippet": query }}));
            body.insert("query".to_string(), json!({ "bool": filter }));

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

    pub(super) async fn upsert_documents(
        &self,
        documents: impl IntoIterator<
            IntoIter = impl ExactSizeIterator<Item = &models::IngestedDocument>,
        >,
    ) -> Result<Warning<DocumentId>, Error> {
        #[derive(Serialize)]
        struct IngestedDocument<'a> {
            snippet: &'a str,
            properties: &'a DocumentProperties,
            embedding: &'a NormalizedEmbedding,
            tags: &'a [DocumentTag],
        }

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
        let url = self.create_url(["_delete_by_query"], [("refresh", NO_PARAM_VALUE)]);
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

    pub(super) async fn upsert_document_properties(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", id.as_ref()], [("refresh", NO_PARAM_VALUE)]);
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
        let url = self.create_url(["_update", id.as_ref()], [("refresh", NO_PARAM_VALUE)]);
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": DocumentProperties::default()
                }
            },
            "_source": false
        }));

        Ok(self
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    pub(super) async fn upsert_document_property(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(
            ["_update", document_id.as_ref()],
            [("refresh", NO_PARAM_VALUE)],
        );
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
        let url = self.create_url(
            ["_update", document_id.as_ref()],
            [("refresh", NO_PARAM_VALUE)],
        );
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

    pub(super) async fn upsert_document_tags(
        &self,
        id: &DocumentId,
        tags: &[DocumentTag],
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
        let url = self.create_url(["_update", id.as_ref()], [("refresh", NO_PARAM_VALUE)]);
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
