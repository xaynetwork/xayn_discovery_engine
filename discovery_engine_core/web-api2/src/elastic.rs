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

use derive_more::From;
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
use xayn_discovery_engine_ai::Embedding;

use crate::{
    error::common::{DocumentIdAsObject, FailedToDeleteSomeDocuments, InternalError},
    models::{DocumentId, DocumentProperties, PersonalizedDocument},
    server::SetupError,
    utils::{serialize_redacted, serialize_to_ndjson},
    Error,
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[allow(dead_code)]
    #[serde(default = "default_url")]
    url: String,
    #[allow(dead_code)]
    #[serde(default = "default_user")]
    user: String,
    #[allow(dead_code)]
    #[serde(default = "default_password", serialize_with = "serialize_redacted")]
    password: Secret<String>,
    #[allow(dead_code)]
    #[serde(default = "default_index_name")]
    index_name: String,
}

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

pub(crate) struct ElasticSearchClient {
    config: Config,
    url_to_index: Url,
    client: reqwest::Client,
}

impl ElasticSearchClient {
    pub(crate) fn new(config: Config) -> Result<Self, SetupError> {
        let mut url_to_index: Url = config.url.parse()?;
        url_to_index
            .path_segments_mut()
            .map_err(|()| "non segmentable url in config")?
            .push(&config.index_name);

        Ok(Self {
            config,
            url_to_index,
            client: reqwest::Client::new(),
        })
    }

    pub(crate) async fn delete_documents(&self, documents: &[DocumentId]) -> Result<(), Error> {
        if documents.is_empty() {
            return Ok(());
        }

        let response = self
            .bulk_request(
                documents
                    .iter()
                    .map(|document_id| Ok(BulkInstruction::Delete { id: document_id })),
            )
            .await?;

        if response.errors {
            let mut errors = Vec::new();
            for mut response in response.items.into_iter() {
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

    pub async fn bulk_insert_documents(
        &self,
        documents: &[(DocumentId, ElasticDocument)],
    ) -> Result<(), BulkInsertionError> {
        let response = self
            .bulk_request(documents.iter().flat_map(|(document_id, document)| {
                [
                    serde_json::to_value(BulkInstruction::Index { id: document_id })
                        .map_err(Into::into),
                    serde_json::to_value(document).map_err(Into::into),
                ]
            }))
            .await?;

        if !response.errors {
            Ok(())
        } else {
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
        }
    }

    async fn bulk_request(
        &self,
        requests: impl IntoIterator<Item = Result<impl Serialize, Error>>,
    ) -> Result<BulkResponse, Error> {
        let mut url = self.create_resource_path(&["_bulk"]);
        url.set_query(Some("refresh"));

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

    fn create_resource_path(&self, segments: &[&str]) -> Url {
        let mut url = self.url_to_index.clone();
        // UNWRAP_SAFE: In the constructor we already made sure it's a segmentable url.
        url.path_segments_mut().unwrap().extend(segments);
        url
    }

    pub(crate) async fn get_documents_by_embedding(
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
            .query_with_json::<_, SearchResponse<_>>(self.create_resource_path(&["_search"]), body)
            .await?
            .map(Into::into)
            .unwrap_or_default())
    }

    pub(crate) async fn get_documents_by_ids(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/query-dsl-ids-query.html
        let body = Some(json!({
            "query": {
                "ids" : {
                    "values" : ids
                }
            }
        }));

        Ok(self
            .query_with_json::<_, SearchResponse<_>>(self.create_resource_path(&["_search"]), body)
            .await?
            .map(Into::into)
            .unwrap_or_default())
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
}

#[derive(From)]
pub(crate) enum BulkInsertionError {
    General(Error),
    PartialFailure {
        failed_documents: Vec<DocumentIdAsObject>,
    },
}

fn is_success_status(status: u16, allow_not_found: bool) -> bool {
    StatusCode::from_u16(status)
        .map(|status| (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success())
        .unwrap_or(false)
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

#[derive(Deserialize)]
struct BulkResponse {
    errors: bool,
    items: Vec<HashMap<String, BulkItemResponse>>,
}

#[derive(Debug, Deserialize)]
struct BulkItemResponse {
    #[serde(rename = "_id")]
    id: DocumentId,
    status: u16,
    #[serde(default)]
    error: Value,
}

pub(crate) struct KnnSearchParams {
    pub(crate) excluded: Vec<DocumentId>,
    pub(crate) embedding: Vec<f32>,
    pub(crate) size: usize,
    pub(crate) k_neighbors: usize,
    pub(crate) num_candidates: usize,
}

/// Represents a document with calculated embeddings that is stored in Elastic Search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElasticDocument {
    pub snippet: String,
    pub properties: DocumentProperties,
    #[serde(with = "serde_embedding_as_vec")]
    pub embedding: Embedding,
}

impl From<SearchResponse<ElasticDocument>> for Vec<PersonalizedDocument> {
    fn from(response: SearchResponse<ElasticDocument>) -> Self {
        response
            .hits
            .hits
            .into_iter()
            .map(|hit| PersonalizedDocument {
                id: hit.id,
                score: hit.score,
                embedding: hit.source.embedding,
                properties: hit.source.properties,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SearchResponse<T> {
    hits: Hits<T>,
}

#[derive(Clone, Debug, Deserialize)]
struct Hits<T> {
    hits: Vec<Hit<T>>,
    #[allow(dead_code)]
    total: Total,
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
struct Total {
    #[allow(dead_code)]
    value: usize,
}

pub(crate) mod serde_embedding_as_vec {
    use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serializer};
    use xayn_discovery_engine_ai::Embedding;

    pub(crate) fn serialize<S>(embedding: &Embedding, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(embedding.len()))?;
        for element in embedding.iter() {
            seq.serialize_element(element)?;
        }
        seq.end()
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Embedding, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<f32>::deserialize(deserializer).map(Embedding::from)
    }
}
