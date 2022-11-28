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

use std::collections::HashMap;

use async_trait::async_trait;
use itertools::Itertools;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Body,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::error;
use xayn_ai_coi::Embedding;

use crate::{
    error::common::{FailedToDeleteSomeDocuments, InternalError},
    models::{
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        IngestedDocument,
        PersonalizedDocument,
    },
    server::SetupError,
    storage::{self, InsertionError, KnnSearchParams, Storage},
    utils::{serialize_redacted, serialize_to_ndjson},
    Error,
};

fn default_url() -> String {
    "http://localhost:9200".into()
}

fn default_user() -> String {
    "elastic".into()
}

fn default_password() -> Secret<String> {
    String::from("changeme").into()
}

fn default_index_name() -> String {
    "test_index".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Config {
    #[serde(default = "default_url")]
    url: String,
    #[serde(default = "default_user")]
    user: String,
    #[serde(default = "default_password", serialize_with = "serialize_redacted")]
    password: Secret<String>,
    #[serde(default = "default_index_name")]
    index_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: default_url(),
            user: default_user(),
            password: default_password(),
            index_name: default_index_name(),
        }
    }
}

impl Config {
    pub(crate) fn setup_client(&self) -> Result<Client, SetupError> {
        let mut url_to_index = self.url.parse::<Url>()?;
        url_to_index
            .path_segments_mut()
            .map_err(|()| "non segmentable url in config")?
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

#[derive(Serialize)]
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

#[derive(Deserialize)]
struct BulkResponse {
    errors: bool,
    items: Vec<HashMap<String, BulkItemResponse>>,
}

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

        self.query_with_bytes::<_, BulkResponse>(url, Some(body), headers)
            .await?
            .ok_or_else(|| InternalError::from_message("_bulk endpoint not found").into())
    }

    async fn query_with_bytes<B, T>(
        &self,
        url: Url,
        body: Option<B>,
        headers: HeaderMap<HeaderValue>,
    ) -> Result<Option<T>, Error>
    where
        B: Into<Body>,
        T: DeserializeOwned,
    {
        let request_builder = if let Some(body) = body {
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

        if response.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            let value = response.error_for_status()?.json().await?;

            Ok(Some(value))
        }
    }

    async fn query_with_json<B, T>(&self, url: Url, body: Option<B>) -> Result<Option<T>, Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let body = body.map(|json| serde_json::to_vec(&json)).transpose()?;

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        self.query_with_bytes(url, body, headers).await
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Hit<T> {
    #[serde(rename = "_id")]
    id: DocumentId,
    #[serde(rename = "_source")]
    source: T,
    #[serde(rename = "_score")]
    score: f32,
}

#[derive(Clone, Debug, Deserialize)]
struct Hits<T> {
    hits: Vec<Hit<T>>,
}

#[derive(Clone, Debug, Deserialize)]
struct SearchResponse<T> {
    hits: Hits<T>,
}

/// Represents a document with calculated embeddings that is stored in Elastic Search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Document {
    pub snippet: String,
    pub properties: DocumentProperties,
    #[serde(with = "serde_embedding_as_vec")]
    pub embedding: Embedding,
    pub category: Option<String>,
}

pub(crate) mod serde_embedding_as_vec {
    use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
    use xayn_ai_coi::Embedding;

    pub(crate) fn serialize<S>(embedding: &Embedding, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(embedding.len()))?;
        for element in embedding.iter() {
            sequence.serialize_element(element)?;
        }
        sequence.end()
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Embedding, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<f32>::deserialize(deserializer).map(Embedding::from)
    }
}

impl From<SearchResponse<Document>> for Vec<PersonalizedDocument> {
    fn from(response: SearchResponse<Document>) -> Self {
        response
            .hits
            .hits
            .into_iter()
            .map(|hit| PersonalizedDocument {
                id: hit.id,
                score: hit.score,
                embedding: hit.source.embedding,
                properties: hit.source.properties,
                category: hit.source.category,
            })
            .collect()
    }
}

fn is_success_status(status: u16, allow_not_found: bool) -> bool {
    StatusCode::from_u16(status)
        .map(|status| (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success())
        .unwrap_or(false)
}

#[async_trait]
impl storage::Document for Storage {
    async fn get_by_ids(&self, ids: &[&DocumentId]) -> Result<Vec<PersonalizedDocument>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/query-dsl-ids-query.html
        let body = Some(json!({
            "query": {
                "ids" : {
                    "values" : ids
                }
            }
        }));

        Ok(self
            .elastic
            .query_with_json::<_, SearchResponse<_>>(
                self.elastic.create_resource_path(["_search"], None),
                body,
            )
            .await?
            .map(Into::into)
            .unwrap_or_default())
    }

    async fn get_by_embedding(
        &self,
        params: KnnSearchParams,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/knn-search.html#approximate-knn
        let body = Some(json!({
            "size": params.size,
            "knn": {
                "field": "embedding",
                "query_vector": params.embedding,
                "k":params.k_neighbors,
                "num_candidates": params.num_candidates,
                "filter": {
                    "bool": {
                        "must_not": {
                            "ids": {
                                "values": params.excluded.iter().map(AsRef::as_ref).collect_vec()
                            }
                        }
                    }
                }
            }
        }));

        Ok(self
            .elastic
            .query_with_json::<_, SearchResponse<_>>(
                self.elastic.create_resource_path(["_search"], None),
                body,
            )
            .await?
            .map(Into::into)
            .unwrap_or_default())
    }

    async fn insert(
        &self,
        documents: Vec<(IngestedDocument, Embedding)>,
    ) -> Result<(), InsertionError> {
        if documents.is_empty() {
            return Ok(());
        }

        let response = self
            .elastic
            .bulk_request(documents.into_iter().flat_map(|(document, embedding)| {
                [
                    serde_json::to_value(BulkInstruction::Index { id: &document.id })
                        .map_err(Into::into),
                    serde_json::to_value(Document {
                        snippet: document.snippet,
                        properties: document.properties,
                        embedding,
                        category: document.category,
                    })
                    .map_err(Into::into),
                ]
            }))
            .await?;

        if response.errors {
            let failed_documents = response.items
                .into_iter()
                .filter_map(|mut response| {
                    if let Some(response) = response.remove("index") {
                        if !is_success_status(response.status, false) {
                            error!(document_id=%response.id, error=%response.error, "Elastic failed to ingest document.");
                            return Some(response.id.into());
                        }
                    } else {
                        error!("Bulk index request contains non index responses: {response:?}");
                    }
                    None
                })
                .collect_vec();

            Err(failed_documents.into())
        } else {
            Ok(())
        }
    }

    async fn delete(&self, documents: &[DocumentId]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let response = self
            .elastic
            .bulk_request(
                documents
                    .iter()
                    .map(|document_id| Ok(BulkInstruction::Delete { id: document_id })),
            )
            .await?;

        if response.errors {
            let mut errors = Vec::new();
            for mut response in response.items {
                if let Some(response) = response.remove("delete") {
                    if !is_success_status(response.status, true) {
                        error!(document_id=%response.id, error=%response.error);
                        errors.push(response.id);
                    }
                } else {
                    error!("Bulk delete request contains non delete responses: {response:?}");
                }
            }

            if !errors.is_empty() {
                return Err(FailedToDeleteSomeDocuments {
                    errors: errors.into_iter().map(Into::into).collect(),
                }
                .into());
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DocumentPropertiesResponse {
    #[serde(default)]
    properties: DocumentProperties,
}

#[derive(Clone, Debug, Deserialize)]
struct IgnoredResponse;

#[async_trait]
impl storage::DocumentProperties for Storage {
    async fn get(&self, id: &DocumentId) -> Result<Option<DocumentProperties>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-get.html
        let url = self.elastic.create_resource_path(
            ["_source", id.as_ref()],
            [("_source_includes", Some("properties"))],
        );

        Ok(self
            .elastic
            .query_with_json::<Value, DocumentPropertiesResponse>(url, None)
            .await?
            .map(|resp| resp.properties))
    }

    async fn put(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let url = self
            .elastic
            .create_resource_path(["_update", id.as_ref()], None);
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
            .elastic
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    async fn delete(&self, id: &DocumentId) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        // don't delete the field, but put an empty map instead, similar to the ingestion service
        let url = self
            .elastic
            .create_resource_path(["_update", id.as_ref()], None);
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
            .elastic
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }
}

#[async_trait]
impl storage::DocumentProperty for Storage {
    async fn get(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<DocumentProperty>>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-get.html
        let url = self.elastic.create_resource_path(
            ["_source", document_id.as_ref()],
            [(
                "_source_includes",
                Some(&*format!("properties.{}", property_id)),
            )],
        );

        Ok(self
            .elastic
            .query_with_json::<Value, DocumentPropertiesResponse>(url, None)
            .await?
            .map(|mut response| response.properties.remove(property_id)))
    }

    async fn put(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let url = self
            .elastic
            .create_resource_path(["_update", document_id.as_ref()], None);
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
            .elastic
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }

    async fn delete(
        &self,
        document_id: &DocumentId,
        property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let url = self
            .elastic
            .create_resource_path(["_update", document_id.as_ref()], None);
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
            .elastic
            .query_with_json::<_, IgnoredResponse>(url, body)
            .await?
            .map(|_| ()))
    }
}