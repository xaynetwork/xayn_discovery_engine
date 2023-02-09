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

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use itertools::{multiunzip, Itertools};
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
    utils::serialize_redacted,
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
            .push("collections")
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

#[derive(Debug, Deserialize)]
struct BulkResponse {
    #[serde(default)]
    status: Value,
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

    async fn bulk_request(&self, request: impl Serialize) -> Result<BulkResponse, Error> {
        let url = self.create_resource_path(["points"], []);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let body = serde_json::to_vec(&request)?;

        self.query_with_bytes::<_, BulkResponse>(url, Some((headers, body)), true)
            .await?
            .ok_or_else(|| InternalError::from_message("_bulk endpoint not found").into())
    }

    async fn query_with_bytes<B, T>(
        &self,
        url: Url,
        post_data: Option<(HeaderMap<HeaderValue>, B)>,
        put: bool,
    ) -> Result<Option<T>, Error>
    where
        B: Into<Body>,
        T: DeserializeOwned,
    {
        let request_builder = if let Some((headers, body)) = post_data {
            if put {
                self.client.put(url).headers(headers).body(body)
            } else {
                self.client.post(url).headers(headers).body(body)
            }
        } else {
            self.client.get(url)
        };

        let response = request_builder
            .header("api-key", self.config.password.expose_secret())
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
        let post_data = body
            .map(|json| -> Result<_, Error> {
                let mut headers = HeaderMap::new();
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let body = serde_json::to_vec(&json)?;
                Ok((headers, body))
            })
            .transpose()?;

        self.query_with_bytes(url, post_data, false).await
    }
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    result: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct PointsResult {
    id: DocumentId,
    payload: Payload,
    vector: NormalizedEmbedding,
}

impl From<Response<PointsResult>> for Vec<models::InteractedDocument> {
    fn from(response: Response<PointsResult>) -> Self {
        response
            .result
            .into_iter()
            .map(|hit| models::InteractedDocument {
                id: hit.id,
                embedding: hit.vector,
                tags: hit.payload.tags,
            })
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    id: DocumentId,
    score: f32,
    payload: Payload,
    vector: NormalizedEmbedding,
}

impl From<Response<SearchResult>> for Vec<models::PersonalizedDocument> {
    fn from(response: Response<SearchResult>) -> Self {
        response
            .result
            .into_iter()
            .map(|hit| models::PersonalizedDocument {
                id: hit.id,
                score: hit.score,
                embedding: hit.vector,
                properties: hit.payload.properties,
                tags: hit.payload.tags,
            })
            .collect()
    }
}

#[derive(Debug, Serialize)]
struct IngestedDocument<'a> {
    snippet: &'a str,
    properties: &'a DocumentProperties,
    embedding: &'a NormalizedEmbedding,
    tags: &'a [DocumentTag],
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
            "ids": values,
            "with_payload": true,
            "with_vector": true,
        }));

        Ok(self
            .elastic
            .query_with_json::<_, Response<_>>(
                self.elastic.create_resource_path(["points"], None),
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
            "ids": values,
            "with_payload": true,
            "with_vector": true,
        }));

        Ok(self
            .elastic
            .query_with_json::<_, Response<_>>(
                self.elastic.create_resource_path(["points"], None),
                body,
            )
            .await?
            .map(Into::into)
            .unwrap_or_default())
    }
}

#[derive(Debug, Serialize)]
struct BatchRequest {
    batch: Batch,
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    snippet: String,
    #[serde(default)]
    properties: DocumentProperties,
    #[serde(default)]
    tags: Vec<DocumentTag>,
}

#[derive(Debug, Serialize)]
struct Batch {
    ids: Vec<DocumentId>,
    vectors: Vec<NormalizedEmbedding>,
    payloads: Vec<Payload>,
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

    async fn get_embedding(&self, _id: &DocumentId) -> Result<Option<NormalizedEmbedding>, Error> {
        unimplemented!()
    }

    async fn get_by_embedding<'a>(
        &self,
        params: KnnSearchParams<'a>,
    ) -> Result<Vec<models::PersonalizedDocument>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/knn-search.html#approximate-knn
        let excluded_ids = params.excluded.iter().map(AsRef::as_ref).collect_vec();

        let filter = if let Some(published_after) = params.published_after {
            // published_after != null && published_after <= publication_date <= now
            json!({
                "bool": {
                    "filter": {
                        "key": "properties.publication_date",
                        "range": {
                            "gte": published_after.timestamp_micros(),
                            "lte": Utc::now().timestamp_micros()
                        }
                    },
                    "must_not": {
                        "has_id": excluded_ids
                    }
                }
            })
        } else {
            // published_after == null || published_after <= now
            json!({
                "must_not": [
                    {
                        "has_id": excluded_ids
                    }
                    //,
                    // {
                    //     "key": "properties.publication_date",
                    //     "range": {
                    //         "gt": Utc::now().timestamp_micros()
                    //     }
                    // }
                ]
            })
        };

        let mut body = json!({
            "vector": params.embedding,
            "filter": filter,
            "limit": params.k_neighbors,
            "params": {
                "hnsw_ef": params.num_candidates,
                "exact": false
            },
            "with_payload": true,
            "with_vector": true,
        });

        if let Some(min_similarity) = params.min_similarity {
            body.as_object_mut()
                .unwrap(/* we just created it as object */)
                .insert("score_threshold".into(), min_similarity.into());
        }

        // the existing documents are not filtered in the elastic query to avoid too much work for a
        // cold path, filtering them afterwards can occasionally lead to less than k results though
        let mut documents = self
            .elastic
            .query_with_json::<_, Response<_>>(
                self.elastic
                    .create_resource_path(["points", "search"], None),
                Some(body),
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
        documents: Vec<(models::IngestedDocument, NormalizedEmbedding)>,
    ) -> Result<(), InsertionError> {
        if documents.is_empty() {
            return Ok(());
        }

        let batch = documents
            .into_iter()
            .map(|(mut document, embedding)| {
                let key = DocumentPropertyId::new("publication_date").unwrap();
                document
                    .properties
                    .remove(&key)
                    .map(|date| {
                        DateTime::parse_from_rfc3339(
                            serde_json::from_value::<String>(date.0).unwrap().as_ref(),
                        )
                        .unwrap()
                        .timestamp_micros()
                    })
                    .and_then(|time| {
                        document
                            .properties
                            .insert(key, DocumentProperty(serde_json::to_value(time).unwrap()))
                    });

                (
                    document.id,
                    embedding,
                    Payload {
                        snippet: document.snippet,
                        properties: document.properties,
                        tags: document.tags,
                    },
                )
            })
            .collect_vec();

        let (ids, vectors, payloads): (Vec<_>, Vec<_>, Vec<_>) = multiunzip(batch);
        let request = BatchRequest {
            batch: Batch {
                ids: ids.clone(),
                vectors,
                payloads,
            },
        };
        let request = serde_json::to_value(request).unwrap();
        let response = self.elastic.bulk_request(request).await?;

        self.postgres.insert_documents(&ids).await?;

        if let Value::Object(_) = response.status {
            error!("{:?}", response.status);
        }

        Ok(())
    }

    async fn delete(&self, _documents: &[DocumentId]) -> Result<(), DeletionError> {
        unimplemented!()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct DocumentPropertiesResponse {
    #[serde(default)]
    _properties: DocumentProperties,
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
    async fn get(&self, _id: &DocumentId) -> Result<Option<DocumentProperties>, Error> {
        unimplemented!()
    }

    async fn put(
        &self,
        _id: &DocumentId,
        _properties: &DocumentProperties,
    ) -> Result<Option<()>, Error> {
        unimplemented!()
    }

    async fn delete(&self, _id: &DocumentId) -> Result<Option<()>, Error> {
        unimplemented!()
    }
}

#[async_trait]
impl storage::DocumentProperty for Storage {
    async fn get(
        &self,
        _document_id: &DocumentId,
        _property_id: &DocumentPropertyId,
    ) -> Result<Option<Option<DocumentProperty>>, Error> {
        unimplemented!()
    }

    async fn put(
        &self,
        _document_id: &DocumentId,
        _property_id: &DocumentPropertyId,
        _property: &DocumentProperty,
    ) -> Result<Option<()>, Error> {
        unimplemented!()
    }

    async fn delete(
        &self,
        _document_id: &DocumentId,
        _property_id: &DocumentPropertyId,
    ) -> Result<Option<()>, Error> {
        unimplemented!()
    }
}
