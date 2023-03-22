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

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use itertools::Itertools;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Body,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use tracing::error;
use xayn_ai_bert::NormalizedEmbedding;

use crate::{
    app::SetupError,
    error::common::InternalError,
    models::{
        self,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentTag,
    },
    storage::{self, DeletionError, InsertionError, KnnSearchParams, Storage},
    utils::{serialize_redacted, serialize_to_ndjson},
    Error,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Config {
    url: String,
    user: String,
    #[serde(serialize_with = "serialize_redacted")]
    password: Secret<String>,
    index_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: "http://localhost:9200".into(),
            user: "elastic".into(),
            password: String::from("changeme").into(),
            index_name: "test_index".into(),
        }
    }
}

impl Config {
    pub(crate) fn setup_client(&self) -> Result<Client, SetupError> {
        let mut url_to_index = self.url.parse::<Url>()?;
        url_to_index
            .path_segments_mut()
            .map_err(|()| anyhow::anyhow!("non segmentable url in config"))?
            .push(&self.index_name);

        Ok(Client {
            config: self.clone(),
            url_to_index,
            client: reqwest::Client::new(),
        })
    }
}

pub(crate) struct Client {
    config: Config,
    url_to_index: Url,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum BulkInstruction<'a> {
    Index {
        #[serde(rename = "_id")]
        id: &'a DocumentId,
    },
    Delete {
        #[serde(rename = "_id")]
        id: &'a DocumentId,
    },
}

#[derive(Debug, Deserialize)]
struct BulkItemResponse {
    #[serde(rename = "_id")]
    id: DocumentId,
    status: u16,
    #[serde(default)]
    error: Value,
}

#[derive(Debug, Deserialize)]
struct BulkResponse {
    errors: bool,
    items: Vec<HashMap<String, BulkItemResponse>>,
}

#[derive(Debug, Deserialize)]
/// Deserializes from any map/struct dropping all fields.
///
/// This will not work with non self describing non schema
/// formats like bincode.
struct IgnoredResponse {/* Note: The {} is needed for it to work correctly. */}

impl Client {
    fn create_resource_path<'a>(
        &self,
        segments: impl IntoIterator<Item = &'a str>,
        query_parts: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    ) -> Url {
        let mut url = self.url_to_index.clone();
        // UNWRAP_SAFE: In the constructor we already made sure it's a segmentable url.
        url.path_segments_mut().unwrap().extend(segments);
        let mut query_mut = url.query_pairs_mut();
        for (key, value) in query_parts {
            if let Some(value) = value {
                query_mut.append_pair(key, value);
            } else {
                query_mut.append_key_only(key);
            }
        }
        drop(query_mut);

        url
    }

    async fn bulk_request(
        &self,
        requests: impl IntoIterator<Item = Result<impl Serialize, Error>>,
    ) -> Result<BulkResponse, Error> {
        let url = self.create_resource_path(["_bulk"], [("refresh", None)]);

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-ndjson"),
        );

        let body = serialize_to_ndjson(requests)?;

        self.query_with_bytes::<_, BulkResponse>(url, Some((headers, body)))
            .await?
            .ok_or_else(|| InternalError::from_message("_bulk endpoint not found").into())
    }

    async fn query_with_bytes<B, T>(
        &self,
        url: Url,
        post_data: Option<(HeaderMap<HeaderValue>, B)>,
    ) -> Result<Option<T>, Error>
    where
        B: Into<Body>,
        T: DeserializeOwned,
    {
        let request_builder = if let Some((headers, body)) = post_data {
            self.client.post(url).headers(headers).body(body)
        } else {
            self.client.get(url)
        };

        let response = request_builder
            .basic_auth(
                &self.config.user,
                Some(self.config.password.expose_secret()),
            )
            .send()
            .await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            Ok(None)
        } else if !status.is_success() {
            let url = response.url().clone();
            let body = response.bytes().await?;
            let err_msg = String::from_utf8_lossy(&body);
            Err(InternalError::from_message(format!(
                "Elastic Search failed, status={status}, url={url}, \nbody={err_msg}"
            ))
            .into())
        } else {
            Ok(Some(response.json().await?))
        }
    }

    async fn query_with_json<B, T>(&self, url: Url, body: Option<B>) -> Result<Option<T>, Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let post_data = body
            .map(|json| -> Result<_, Error> {
                let mut headers = HeaderMap::new();
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let body = serde_json::to_vec(&json)?;
                Ok((headers, body))
            })
            .transpose()?;

        self.query_with_bytes(url, post_data).await
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

fn is_success_status(status: u16, allow_not_found: bool) -> bool {
    StatusCode::from_u16(status)
        .map(|status| (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success())
        .unwrap_or(false)
}

#[async_trait(?Send)]
impl storage::Document for Storage {
    async fn get_interacted(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<models::InteractedDocument>, Error> {
        self.get_interacted(ids).await
    }

    async fn get_personalized(
        &self,
        ids: impl IntoIterator<IntoIter = impl ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        self.get_personalized(ids, |_| Some(1.0)).await
    }

    async fn get_embedding(&self, id: &DocumentId) -> Result<Option<NormalizedEmbedding>, Error> {
        self.get_embedding(id).await
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a, impl IntoIterator<Item = &'a DocumentId>>,
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        // the existing documents are not filtered in the elastic query to avoid too much work for a
        // cold path, filtering them afterwards can occasionally lead to less than k results though
        let excluded_ids = json!({
            "values": params.excluded.into_iter().collect_vec()
        });
        let time = params.time.to_rfc3339();
        let filter = if let Some(published_after) = params.published_after {
            // published_after != null && published_after <= publication_date <= time
            json!({
                "bool": {
                    "filter": {
                        "range": {
                            "properties.publication_date": {
                                "gte": published_after.to_rfc3339(),
                                "lte": time
                            }
                        }
                    },
                    "must_not": {
                        "ids": excluded_ids
                    }
                }
            })
        } else {
            // published_after == null || published_after <= time
            json!({
                "bool": {
                    "must_not": [
                        {
                            "ids": excluded_ids
                        },
                        {
                            "range": {
                                "properties.publication_date": {
                                    "gt": time
                                }
                            }
                        }
                    ]
                }
            })
        };

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/knn-search.html#approximate-knn
        let mut body = json!({
            "knn": {
                "field": "embedding",
                "query_vector": params.embedding,
                "k": params.k_neighbors,
                "num_candidates": params.num_candidates,
                "filter": filter
            },
            "size": params.k_neighbors,
            "_source": false
        });
        if let Some(min_similarity) = params.min_similarity {
            body.as_object_mut()
                .unwrap(/* we just created it as object */)
                .insert("min_score".into(), min_similarity.into());
        }

        let scores = self
            .elastic
            .query_with_json::<_, SearchResponse<NoSource>>(
                self.elastic.create_resource_path(["_search"], None),
                Some(body),
            )
            .await?
            .map(|response| {
                response
                    .hits
                    .hits
                    .into_iter()
                    .map(|hit| (hit.id, hit.score))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();

        self.get_personalized(scores.keys(), |id| scores.get(id).copied())
            .await
    }

    async fn insert(
        &self,
        documents: Vec<(models::IngestedDocument, NormalizedEmbedding)>,
    ) -> Result<(), InsertionError> {
        if documents.is_empty() {
            return Ok(());
        }

        let mut ids = documents
            .iter()
            .map(|(document, _)| document.id.clone())
            .collect::<HashSet<_>>();

        let response = self
            .elastic
            .bulk_request(documents.iter().flat_map(|(document, embedding)| {
                [
                    serde_json::to_value(BulkInstruction::Index { id: &document.id })
                        .map_err(Into::into),
                    serde_json::to_value(IngestedDocument {
                        snippet: &document.snippet,
                        properties: &document.properties,
                        embedding,
                        tags: &document.tags,
                    })
                    .map_err(Into::into),
                ]
            }))
            .await?;
        let failed_documents = response.errors.then(|| {
            response
                .items
                .into_iter()
                .filter_map(|mut response| {
                    if let Some(response) = response.remove("index") {
                        if !is_success_status(response.status, false) {
                            error!(
                                document_id=%response.id,
                                error=%response.error,
                                "Elastic failed to ingest document.",
                            );
                            ids.remove(&response.id);
                            return Some(response.id.into());
                        }
                    } else {
                        error!("Bulk index request contains non index responses: {response:?}");
                    }
                    None
                })
                .collect_vec()
        });

        self.postgres
            .insert_documents(
                documents
                    .iter()
                    .filter(|(document, _)| ids.contains(&document.id)),
            )
            .await?;

        if let Some(failed_documents) = failed_documents {
            Err(failed_documents.into())
        } else {
            Ok(())
        }
    }

    async fn delete(
        &self,
        ids: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
    ) -> Result<(), DeletionError> {
        let ids = ids.into_iter();
        if ids.len() == 0 {
            return Ok(());
        }

        let mut failed_documents = match self.postgres.delete_documents(ids.clone()).await {
            Ok(()) => Vec::new(),
            Err(DeletionError::PartialFailure { errors }) => errors,
            Err(error) => return Err(error),
        };

        let response = self
            .elastic
            .bulk_request(ids.map(|id| Ok(BulkInstruction::Delete { id })))
            .await?;
        if response.errors {
            for mut response in response.items {
                if let Some(response) = response.remove("delete") {
                    if !is_success_status(response.status, true) {
                        error!(
                            document_id=%response.id,
                            error=%response.error,
                            "Elastic failed to delete document.",
                        );
                        failed_documents.push(response.id.into());
                    }
                } else {
                    error!("Bulk delete request contains non delete responses: {response:?}");
                }
            }
        }

        if failed_documents.is_empty() {
            Ok(())
        } else {
            Err(failed_documents.into())
        }
    }
}
