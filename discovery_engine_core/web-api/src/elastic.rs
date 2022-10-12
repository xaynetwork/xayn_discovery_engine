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

use itertools::Itertools;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_discovery_engine_ai::Embedding;

use crate::models::{DocumentId, DocumentProperties, Error, PersonalizedDocumentData};

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
        let body = json!({
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
        });

        let response = self.query_elastic_search(body).await?;
        Ok(convert_response(response))
    }

    pub(crate) async fn get_documents_by_ids(
        &self,
        ids: &[&DocumentId],
    ) -> Result<Vec<PersonalizedDocumentData>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/query-dsl-ids-query.html
        let body = json!({
            "query": {
                "ids" : {
                    "values" : ids
                }
            }
        });

        let response = self.query_elastic_search(body).await?;
        Ok(convert_response(response))
    }

    async fn query_elastic_search(
        &self,
        body: Value,
    ) -> Result<Response<ElasticDocumentData>, Error> {
        let url = format!("{}/{}/_search", self.config.url, self.config.index_name);

        let res = self
            .client
            .post(url)
            .basic_auth(&self.config.user, Some(&self.config.password))
            .json(&body)
            .send()
            .await
            .map_err(Error::Elastic)?
            .error_for_status()
            .map_err(Error::Elastic)?;

        res.json().await.map_err(Error::Receiving)
    }
}

fn convert_response(response: Response<ElasticDocumentData>) -> Vec<PersonalizedDocumentData> {
    response
        .hits
        .hits
        .into_iter()
        .map(|hit| PersonalizedDocumentData {
            id: DocumentId(hit.id),
            score: hit.score,
            embedding: hit.source.embedding,
            properties: hit.source.properties,
        })
        .collect()
}

/// Represents a document with calculated embeddings that is stored in Elastic Search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticDocumentData {
    pub snippet: String,
    pub properties: DocumentProperties,
    #[serde(with = "serde_embedding_as_vec")]
    pub embedding: Embedding,
}

#[derive(Clone, Deserialize, Debug)]
#[allow(dead_code)]
struct Response<T> {
    hits: Hits<T>,
}

#[derive(Clone, Deserialize, Debug)]
#[allow(dead_code)]
struct Hits<T> {
    hits: Vec<Hit<T>>,
    total: Total,
}

#[derive(Clone, Deserialize, Debug)]
#[allow(dead_code)]
struct Hit<T> {
    #[serde(rename(deserialize = "_id"))]
    id: String,
    #[serde(rename(deserialize = "_source"))]
    source: T,
    #[serde(rename(deserialize = "_score"))]
    score: f32,
}

#[derive(Clone, Deserialize, Debug)]
#[allow(dead_code)]
struct Total {
    value: usize,
}

pub(crate) mod serde_embedding_as_vec {
    use ndarray::Array;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use xayn_discovery_engine_ai::Embedding;

    pub(crate) fn serialize<S>(embedding: &Embedding, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        embedding.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Embedding, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<f32>::deserialize(deserializer).map(|vec| Embedding::from(Array::from_vec(vec)))
    }
}
