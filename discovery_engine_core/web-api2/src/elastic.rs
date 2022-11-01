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

use reqwest::{
    header::{HeaderValue, CONTENT_TYPE},
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::error;

use crate::{
    error::common::{FailedToDeleteSomeDocuments, InternalError},
    models::DocumentId,
    server::SetupError,
    utils::serialize_redacted,
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
                    .map(|document_id| json!({ "delete": { "_id": document_id }})),
            )
            .await?;

        if response.errors {
            let mut errors = Vec::new();
            for mut response in response.items.into_iter() {
                if let Some(response) = response.remove("delete") {
                    if let Ok(status) = StatusCode::from_u16(response.status) {
                        if status != StatusCode::NOT_FOUND && !status.is_success() {
                            error!(document_id=%response.id, error=%response.error);
                            errors.push(response.id);
                        }
                    } else {
                        error!("Non http status code: {}", response.status);
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

    async fn bulk_request(
        &self,
        requests: impl IntoIterator<Item = Value>,
    ) -> Result<BulkResponse, Error> {
        let url = self.create_resource_path(&["_bulk"]);

        let mut body = Vec::new();
        for request in requests {
            serde_json::to_writer(&mut body, &request)?;
            body.push(b'\n');
        }

        let response: BulkResponse = self
            .client
            .post(url)
            .header(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-ndjson"),
            )
            .body(body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response)
    }

    fn create_resource_path(&self, segments: &[&str]) -> Url {
        let mut url = self.url_to_index.clone();
        // UNWRAP_SAFE: In the constructor we already made sure it's a segmentable url.
        url.path_segments_mut().unwrap().extend(segments);
        url
    }

    #[allow(dead_code)]
    async fn query_elastic_search<B, T>(
        &self,
        route: &str,
        body: Option<B>,
    ) -> Result<Option<T>, Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let url = format!("{}/{}/{}", self.config.url, self.config.index_name, route);

        let request_builder = if let Some(body) = body {
            self.client
                .post(url)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
                .json(&body)
        } else {
            self.client.get(url)
        };

        let response = request_builder
            .basic_auth(
                &self.config.user,
                Some(self.config.password.expose_secret()),
            )
            .send()
            .await
            .map_err(InternalError::from_std)?;

        if response.status() == StatusCode::NOT_FOUND {
            Ok(None)
        } else {
            let value = response
                .error_for_status()
                .map_err(InternalError::from_std)?
                .json()
                .await
                .map_err(InternalError::from_std)?;

            Ok(Some(value))
        }
    }
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
