// Copyright 2023 Xayn AG
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

use std::{collections::HashMap, fmt::Debug, str::FromStr, sync::Arc};

use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Body,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tracing::error;

use crate::{
    app::SetupError,
    error::common::InternalError,
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

#[derive(Clone)]
pub(crate) struct Client {
    config: Arc<Config>,
    url_to_index: Url,
    client: reqwest::Client,
}

impl Client {
    pub(crate) fn builder(config: &Config) -> Result<ClientBuilder, SetupError> {
        Ok(ClientBuilder {
            config: Arc::new(config.clone()),
            base_url: Arc::new(config.url.parse::<SegmentableUrl>()?),
            client: reqwest::Client::new(),
        })
    }
}

#[derive(Clone)]
pub(crate) struct ClientBuilder {
    config: Arc<Config>,
    base_url: Arc<SegmentableUrl>,
    client: reqwest::Client,
}

impl ClientBuilder {
    pub(crate) fn build(&self) -> Client {
        Client {
            config: self.config.clone(),
            url_to_index: self
                .base_url
                .with_segments([&self.config.index_name])
                .into(),
            client: self.client.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum BulkInstruction<'a, D> {
    Index {
        #[serde(rename = "_id")]
        id: &'a D,
    },
    Delete {
        #[serde(rename = "_id")]
        id: &'a D,
    },
}

#[derive(Debug, Deserialize)]
pub(super) struct BulkItemResponse<D> {
    #[serde(rename = "_id")]
    pub(super) id: D,
    pub(super) status: u16,
    #[serde(default)]
    pub(super) error: Value,
}

impl<D> BulkItemResponse<D> {
    fn is_success_status(&self, allow_not_found: bool) -> bool {
        StatusCode::from_u16(self.status)
            .map(|status| {
                (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct BulkResponse<D> {
    pub(super) errors: bool,
    pub(super) items: Vec<HashMap<String, BulkItemResponse<D>>>,
}

impl<D> BulkResponse<D> {
    pub(super) fn failed_documents(self, operation: &'static str, allow_not_found: bool) -> Vec<D>
    where
        D: Debug,
    {
        self.errors.then(|| {
            self
                .items
                .into_iter()
                .filter_map(|mut response| {
                    if let Some(response) = response.remove(operation) {
                        if !response.is_success_status(allow_not_found) {
                            error!(
                                document_id=?response.id,
                                error=%response.error,
                                "Elastic failed to {operation} document.",
                            );
                            return Some(response.id);
                        }
                    } else {
                        error!("Bulk {operation} request contains non {operation} responses: {response:?}");
                    }
                    None
                })
                .collect()
        }).unwrap_or_default()
    }
}

impl Client {
    pub(crate) fn create_resource_path<'a>(
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

    pub(super) async fn bulk_request<D>(
        &self,
        requests: impl IntoIterator<Item = Result<impl Serialize, Error>>,
    ) -> Result<BulkResponse<D>, Error>
    where
        D: DeserializeOwned,
    {
        let url = self.create_resource_path(["_bulk"], [("refresh", None)]);

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-ndjson"),
        );

        let body = serialize_to_ndjson(requests)?;

        self.query_with_bytes::<_, BulkResponse<D>>(url, Some((headers, body)))
            .await?
            .ok_or_else(|| InternalError::from_message("_bulk endpoint not found").into())
    }

    pub(super) async fn query_with_bytes<B, T>(
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

    pub(super) async fn query_with_json<B, T>(
        &self,
        url: Url,
        body: Option<B>,
    ) -> Result<Option<T>, Error>
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
}

#[derive(derive_more::Into, Clone)]
struct SegmentableUrl(Url);

impl SegmentableUrl {
    fn with_segments(&self, segments: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut new_url = self.0.clone();
        let mut segments_mut = new_url.path_segments_mut()
            .unwrap(/* we made sure this can't happen */);
        for segment in segments {
            segments_mut.push(segment.as_ref());
        }
        drop(segments_mut);
        Self(new_url)
    }
}

impl TryFrom<Url> for SegmentableUrl {
    type Error = anyhow::Error;

    fn try_from(mut url: Url) -> Result<Self, Self::Error> {
        url.path_segments_mut()
            .map_err(|()| anyhow::anyhow!("non segmentable url"))?;
        Ok(Self(url))
    }
}

impl FromStr for SegmentableUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Url>()?.try_into()
    }
}
