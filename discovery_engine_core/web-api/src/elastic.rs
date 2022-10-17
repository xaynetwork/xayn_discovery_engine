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

use itertools::Itertools;
use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_discovery_engine_ai::Embedding;

use crate::models::{
    DocumentId,
    DocumentProperties,
    DocumentProperty,
    DocumentPropertyId,
    Error,
    PersonalizedDocumentData,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub url: String,
    pub index_name: String,
    pub user: String,
    pub password: String,
}

pub(crate) struct ElasticState {
    config: Config,
    client: Client,
}

pub(crate) struct KnnSearchParams {
    pub(crate) excluded: Vec<DocumentId>,
    pub(crate) embedding: Vec<f32>,
    pub(crate) size: usize,
    pub(crate) k_neighbors: usize,
    pub(crate) num_candidates: usize,
}

#[allow(dead_code)]
impl ElasticState {
    pub(crate) fn new(config: Config) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    pub(crate) async fn get_documents_by_embedding(
        &self,
        params: KnnSearchParams,
    ) -> Result<Vec<PersonalizedDocumentData>, Error> {
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

        self.query_elastic_search::<SearchResponse<_>>("_search", body)
            .await
            .map(Into::into)
    }

    pub(crate) async fn get_documents_by_ids(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<PersonalizedDocumentData>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/query-dsl-ids-query.html
        let body = Some(json!({
            "query": {
                "ids" : {
                    "values" : ids
                }
            }
        }));

        self.query_elastic_search::<SearchResponse<_>>("_search", body)
            .await
            .map(Into::into)
    }

    pub(crate) async fn get_document_properties(
        &self,
        id: &DocumentId,
    ) -> Result<DocumentProperties, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-get.html
        self.query_elastic_search::<DocumentPropertiesResponse>(
            &format!("_source/{id}?_source_includes=properties"),
            None,
        )
        .await
        .map(|response| response.properties)
    }

    pub(crate) async fn put_document_properties(
        &self,
        id: &DocumentId,
        properties: &DocumentProperties,
    ) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": properties
                }
            },
            "_source": false
        }));
        self.query_elastic_search::<GenericResponse>(&format!("_update/{id}"), body)
            .await?;

        Ok(())
    }

    pub(crate) async fn delete_document_properties(&self, id: &DocumentId) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        // don't delete the field, but put an empty map instead, similar to the ingestion service
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties = params.properties",
                "params": {
                    "properties": DocumentProperties::new()
                }
            },
            "_source": false
        }));
        self.query_elastic_search::<GenericResponse>(&format!("_update/{id}"), body)
            .await?;

        Ok(())
    }

    pub(crate) async fn get_document_property(
        &self,
        doc_id: &DocumentId,
        prop_id: &DocumentPropertyId,
    ) -> Result<Option<DocumentProperty>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-get.html
        self.query_elastic_search::<DocumentPropertyResponse>(
            &format!("_source/{doc_id}?_source_includes=properties.{prop_id}"),
            None,
        )
        .await
        .map(|mut response| response.0.remove(prop_id))
    }

    pub(crate) async fn put_document_property(
        &self,
        doc_id: &DocumentId,
        prop_id: &DocumentPropertyId,
        property: &DocumentProperty,
    ) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties.put(params.prop_id, params.property)",
                "params": {
                    "prop_id": prop_id,
                    "property": property
                }
            },
            "_source": false
        }));
        self.query_elastic_search::<GenericResponse>(&format!("_update/{doc_id}"), body)
            .await?;

        Ok(())
    }

    pub(crate) async fn delete_document_property(
        &self,
        doc_id: &DocumentId,
        prop_id: &DocumentPropertyId,
    ) -> Result<(), Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/docs-update.html
        let body = Some(json!({
            "script": {
                "source": "ctx._source.properties.remove(params.prop_id)",
                "params": {
                    "prop_id": prop_id
                }
            },
            "_source": false
        }));
        self.query_elastic_search::<GenericResponse>(&format!("_update/{doc_id}"), body)
            .await?;

        Ok(())
    }

    async fn query_elastic_search<T>(&self, route: &str, body: Option<Value>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/{}/{}", self.config.url, self.config.index_name, route);

        if let Some(body) = body {
            self.client
                .post(url)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .json(&body)
        } else {
            self.client.get(url)
        }
        .basic_auth(&self.config.user, Some(&self.config.password))
        .send()
        .await
        .map_err(Error::Elastic)?
        .error_for_status()
        .map_err(Error::Elastic)?
        .json()
        .await
        .map_err(Error::Receiving)
    }
}

/// Represents a document with calculated embeddings that is stored in Elastic Search.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElasticDocumentData {
    pub snippet: String,
    pub properties: DocumentProperties,
    #[serde(with = "serde_embedding_as_vec")]
    pub embedding: Embedding,
}

impl From<SearchResponse<ElasticDocumentData>> for Vec<PersonalizedDocumentData> {
    fn from(response: SearchResponse<ElasticDocumentData>) -> Self {
        response
            .hits
            .hits
            .into_iter()
            .map(|hit| PersonalizedDocumentData {
                id: hit.id,
                score: hit.score,
                embedding: hit.source.embedding,
                properties: hit.source.properties,
            })
            .collect()
    }
}

type GenericResponse = HashMap<String, serde_json::Value>;

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

#[derive(Clone, Debug, Deserialize)]
struct DocumentPropertiesResponse {
    #[serde(default)]
    properties: DocumentProperties,
}

#[derive(Clone, Debug, Deserialize)]
struct DocumentPropertyResponse(DocumentProperties);

pub(crate) mod serde_embedding_as_vec {
    use ndarray::Array;
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
        Vec::<f32>::deserialize(deserializer).map(|vec| Embedding::from(Array::from_vec(vec)))
    }
}
