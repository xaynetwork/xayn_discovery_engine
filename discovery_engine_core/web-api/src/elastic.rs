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

use ndarray::Array;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use xayn_discovery_engine_ai::Embedding;

use crate::models::{DocumentId, DocumentProperties, Error, PersonalizedDocument};

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

#[allow(dead_code)]
impl ElasticState {
    pub(crate) fn new(config: Config) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    pub(crate) async fn get_documents_by_embedding(
        &self,
        embedding: Embedding,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        // https://www.elastic.co/guide/en/elasticsearch/reference/8.4/knn-search.html#approximate-knn
        let body = json!({
            "knn": {
                "field": "embedding",
                "query_vector": embedding.to_vec(),
                // TODO: make below configurable
                "k": 10,
                "num_candidates": 100,
            }
        });

        let response = self.query_elastic_search(body).await?;
        Ok(convert_response(response))
    }

    pub(crate) async fn get_documents_by_ids(
        &self,
        ids: Vec<String>,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
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
        let url = format!("{}/{}/_search", self.config.url, self.config.index_name,);

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

fn convert_response(response: Response<ElasticDocumentData>) -> Vec<PersonalizedDocument> {
    response
        .hits
        .hits
        .into_iter()
        .map(|hit| PersonalizedDocument {
            id: DocumentId(hit.id),
            score: hit.score,
            embedding: Embedding::from(Array::from_vec(hit.source.embedding)),
            properties: hit.source.properties,
        })
        .collect()
}

/// Represents a document with calculated embeddings that is stored in Elastic Search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElasticDocumentData {
    pub snippet: String,
    pub properties: DocumentProperties,
    pub embedding: Vec<f32>,
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