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
use chrono::Utc;
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
use sqlx::{Postgres, Transaction};
use tracing::error;
use xayn_ai_coi::Embedding;

use crate::{
    error::common::InternalError,
    models::{self, DocumentId, DocumentProperties, DocumentProperty, DocumentPropertyId},
    server::SetupError,
    storage::{self, DeletionError, InsertionError, KnnSearchParams, Storage},
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

#[derive(Debug, Deserialize)]
struct Hit<T> {
    #[serde(rename = "_id")]
    id: DocumentId,
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

#[derive(Debug, Deserialize)]
struct InteractedDocument {
    embedding: Embedding,
    #[serde(default)]
    tags: Vec<String>,
}

impl From<SearchResponse<InteractedDocument>> for Vec<models::InteractedDocument> {
    fn from(response: SearchResponse<InteractedDocument>) -> Self {
        response
            .hits
            .hits
            .into_iter()
            .map(|hit| models::InteractedDocument {
                id: hit.id,
                embedding: hit.source.embedding,
                tags: hit.source.tags,
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct PersonalizedDocument {
    #[serde(default)]
    properties: DocumentProperties,
    embedding: Embedding,
    #[serde(default)]
    tags: Vec<String>,
}

impl From<SearchResponse<PersonalizedDocument>> for Vec<models::PersonalizedDocument> {
    fn from(response: SearchResponse<PersonalizedDocument>) -> Self {
        response
            .hits
            .hits
            .into_iter()
            .map(|hit| models::PersonalizedDocument {
                id: hit.id,
                score: hit.score,
                embedding: hit.source.embedding,
                properties: hit.source.properties,
                tags: hit.source.tags,
            })
            .collect()
    }
}

#[derive(Debug, Serialize)]
struct IngestedDocument<'a> {
    snippet: &'a str,
    properties: &'a DocumentProperties,
    embedding: &'a Embedding,
    tags: &'a [String],
}

fn is_success_status(status: u16, allow_not_found: bool) -> bool {
    StatusCode::from_u16(status)
        .map(|status| (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success())
        .unwrap_or(false)
}

impl Storage {
    pub(crate) async fn get_interacted_with_transaction(
        &self,
        ids: &[&DocumentId],
        tx: Option<&mut Transaction<'_, Postgres>>,
    ) -> Result<Vec<models::InteractedDocument>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let values = if let Some(tx) = tx {
            self.postgres
                .documents_exist_with_transaction(ids, tx)
                .await?
        } else {
            self.postgres.documents_exist(ids).await?
        };

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
        let body = Some(json!({
            "query": {
                "ids" : {
                    "values": values
                }
            },
            "_source": ["embedding", "tags"]
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

    pub(crate) async fn get_personalized_with_transaction(
        &self,
        ids: &[&DocumentId],
        tx: Option<&mut Transaction<'_, Postgres>>,
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let values = if let Some(tx) = tx {
            self.postgres
                .documents_exist_with_transaction(ids, tx)
                .await?
        } else {
            self.postgres.documents_exist(ids).await?
        };

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-ids-query.html
        let body = Some(json!({
            "query": {
                "ids" : {
                    "values": values
                }
            },
            "_source": ["properties", "embedding", "tags"]
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
}

#[async_trait]
impl storage::Document for Storage {
    async fn get_interacted(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<models::InteractedDocument>, Error> {
        self.get_interacted_with_transaction(ids, None).await
    }

    async fn get_personalized(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        self.get_personalized_with_transaction(ids, None).await
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/knn-search.html#approximate-knn
        let mut range = serde_json::Map::default();
        range.insert("lte".into(), "now/d".into());
        if let Some(published_after) = params.published_after {
            range.insert("gte".into(), published_after.to_rfc3339().into());
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/knn-search.html#approximate-knn
        let body = Some(json!({
            "size": params.k_neighbors,
            "knn": {
                "field": "embedding",
                "query_vector": params.embedding.normalize()?,
                "k": params.k_neighbors,
                "num_candidates": params.num_candidates,
                "filter": {
                    "bool": {
                        "must": {
                            "range": {
                                "publication_date": range,
                            }
                        },
                        "must_not": {
                            "ids": {
                                "values": params.excluded.iter().map(AsRef::as_ref).collect_vec()
                            }
                        },
                    }
                }
            },
            "_source": ["properties", "embedding", "tags"]
        }));

        // the existing documents are not filtered in the elastic query to avoid too much work for a
        // cold path, filtering them afterwards can occasionally lead to less than k results though
        let mut documents = self
            .elastic
            .query_with_json::<_, SearchResponse<_>>(
                self.elastic.create_resource_path(["_search"], None),
                body,
            )
            .await?
            .map(<Vec<models::PersonalizedDocument>>::from)
            .unwrap_or_default();
        let ids = self
            .postgres
            .documents_exist(&documents.iter().map(|document| &document.id).collect_vec())
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
        documents.retain(|document| ids.contains(&document.id));

        Ok(documents)
    }

    async fn insert(
        &self,
        documents: Vec<(models::IngestedDocument, Embedding)>,
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
            .bulk_request(documents.into_iter().flat_map(|(document, embedding)| {
                [
                    serde_json::to_value(BulkInstruction::Index { id: &document.id })
                        .map_err(Into::into),
                    serde_json::to_value(IngestedDocument {
                        snippet: &document.snippet,
                        properties: &document.properties,
                        embedding: &embedding,
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
            .insert_documents(&ids.into_iter().collect_vec(/* Itertools::chunks() is !Send */))
            .await?;

        if let Some(failed_documents) = failed_documents {
            Err(failed_documents.into())
        } else {
            Ok(())
        }
    }

    async fn delete(&self, documents: &[DocumentId]) -> Result<(), DeletionError> {
        if documents.is_empty() {
            return Ok(());
        }

        self.postgres.delete_documents(documents).await?;

        let response = self
            .elastic
            .bulk_request(
                documents
                    .iter()
                    .map(|document_id| Ok(BulkInstruction::Delete { id: document_id })),
            )
            .await?;
        let failed_documents = response.errors.then(|| {
            response
                .items
                .into_iter()
                .filter_map(|mut response| {
                    if let Some(response) = response.remove("delete") {
                        if !is_success_status(response.status, true) {
                            error!(
                                document_id=%response.id,
                                error=%response.error,
                                "Elastic failed to delete document.",
                            );
                            return Some(response.id.into());
                        }
                    } else {
                        error!("Bulk delete request contains non delete responses: {response:?}",);
                    }
                    None
                })
                .collect_vec()
        });

        if let Some(failed_documents) = failed_documents {
            Err(failed_documents.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DocumentPropertiesResponse {
    #[serde(default)]
    properties: DocumentProperties,
}

#[derive(Clone, Debug, Deserialize)]
/// Deserializes from any map/struct dropping all fields.
///
/// This will not work with non self describing non schema
/// formats like bincode.
//Note: The {} is needed for it to work correctly.
struct IgnoredResponse {}

#[async_trait]
impl storage::DocumentProperties for Storage {
    async fn get(&self, id: &DocumentId) -> Result<Option<DocumentProperties>, Error> {
        if !self.postgres.document_exists(id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-get.html
        let url = self
            .elastic
            .create_resource_path(["_source", id.as_ref()], [("_source", Some("properties"))]);

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
        if !self.postgres.document_exists(id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
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
        if !self.postgres.document_exists(id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
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
        if !self.postgres.document_exists(document_id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-get.html
        let url = self.elastic.create_resource_path(
            ["_source", document_id.as_ref()],
            [("_source", Some(&*format!("properties.{property_id}")))],
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
        if !self.postgres.document_exists(document_id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
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
        if !self.postgres.document_exists(document_id).await? {
            return Ok(None);
        }

        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-update.html
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
