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

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use derive_more::From;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Body,
    Method,
    RequestBuilder,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::error;

use crate::serde::{serde_duration_as_seconds, serialize_redacted, serialize_to_ndjson};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub url: String,
    pub user: String,
    #[serde(serialize_with = "serialize_redacted")]
    pub password: Secret<String>,
    pub index_name: String,

    /// Request timeout in seconds.
    #[serde(with = "serde_duration_as_seconds")]
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: "http://localhost:9200".into(),
            user: "elastic".into(),
            password: String::from("changeme").into(),
            index_name: "test_index".into(),
            timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug)]
struct Auth {
    user: String,
    password: Secret<String>,
}

impl Auth {
    fn apply_to(&self, request: RequestBuilder) -> RequestBuilder {
        request.basic_auth(&self.user, Some(self.password.expose_secret()))
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    auth: Arc<Auth>,
    url_to_index: Arc<SegmentableUrl>,
    client: reqwest::Client,
}

impl Client {
    pub fn new(config: Config) -> Result<Self, anyhow::Error> {
        let Config {
            url,
            user,
            password,
            index_name,
            timeout,
        } = config;
        Ok(Self {
            auth: Auth { user, password }.into(),
            url_to_index: url
                .parse::<SegmentableUrl>()?
                .with_segments([&index_name])
                .into(),
            client: reqwest::ClientBuilder::new().timeout(timeout).build()?,
        })
    }

    /// Sets a different index.
    ///
    /// This will not change the default index configured in the config.
    pub fn with_index(&self, index: impl AsRef<str>) -> Self {
        Self {
            auth: self.auth.clone(),
            url_to_index: self
                .url_to_index
                .with_replaced_last_segment(index.as_ref())
                .into(),
            client: self.client.clone(),
        }
    }

    pub fn request<'a>(
        &self,
        method: Method,
        segments: impl IntoIterator<Item = &'a str>,
        query_parts: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    ) -> RequestBuilder {
        let url = self.create_url(segments, query_parts);
        let builder = self.client.request(method, url);
        self.auth.apply_to(builder)
    }

    pub fn create_url<'a>(
        &self,
        segments: impl IntoIterator<Item = &'a str>,
        query_parts: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
    ) -> Url {
        let mut url: Url = self.url_to_index.with_segments(segments).into();
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BulkInstruction<'a, I> {
    Index {
        #[serde(rename = "_id")]
        id: &'a I,
    },
    Delete {
        #[serde(rename = "_id")]
        id: &'a I,
    },
}

#[derive(Debug, Deserialize)]
pub struct BulkItemResponse<I> {
    #[serde(rename = "_id")]
    pub id: I,
    pub status: u16,
    #[serde(default)]
    pub error: Value,
}

impl<I> BulkItemResponse<I> {
    fn is_success_status(&self, allow_not_found: bool) -> bool {
        StatusCode::from_u16(self.status)
            .map(|status| {
                (status == StatusCode::NOT_FOUND && allow_not_found) || status.is_success()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct BulkResponse<I> {
    pub errors: bool,
    pub items: Vec<HashMap<String, BulkItemResponse<I>>>,
}

impl<I> BulkResponse<I> {
    pub fn failed_documents(self, operation: &'static str, allow_not_found: bool) -> Vec<I>
    where
        I: Display + Debug,
    {
        self.errors.then(|| {
            self
                .items
                .into_iter()
                .filter_map(|mut response| {
                    if let Some(response) = response.remove(operation) {
                        if !response.is_success_status(allow_not_found) {
                            error!(
                                document_id=%response.id,
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

#[derive(Debug, Deserialize)]
struct Hit<I, T> {
    #[serde(rename = "_id")]
    id: I,
    #[allow(dead_code)]
    #[serde(rename = "_source")]
    source: T,
    #[serde(rename = "_score")]
    score: f32,
}

#[derive(Debug, Deserialize)]
struct Hits<I, T> {
    hits: Vec<Hit<I, T>>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse<I, T> {
    hits: Hits<I, T>,
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

impl Client {
    pub async fn bulk_request<I>(
        &self,
        requests: impl IntoIterator<Item = Result<impl Serialize, serde_json::Error>>,
    ) -> Result<BulkResponse<I>, Error>
    where
        I: DeserializeOwned,
    {
        // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
        let url = self.create_url(["_bulk"], [("refresh", None)]);

        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-ndjson"),
        );

        let body = serialize_to_ndjson(requests)?;

        self.query_with_bytes::<_, BulkResponse<I>>(url, Some((headers, body)))
            .await?
            .ok_or_else(|| Error::EndpointNotFound("_bulk"))
    }

    pub async fn search_request<I>(&self, body: impl Serialize) -> Result<HashMap<I, f32>, Error>
    where
        I: DeserializeOwned + Eq + Hash,
    {
        self.query_with_json::<_, SearchResponse<I, NoSource>>(
            self.create_url(["_search"], None),
            Some(body),
        )
        .await
        .map(|response| {
            response
                .map(|response| {
                    response
                        .hits
                        .hits
                        .into_iter()
                        .map(|hit| (hit.id, hit.score))
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    pub async fn query_with_bytes<B, T>(
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

        let response = self.auth.apply_to(request_builder).send().await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            Ok(None)
        } else if !status.is_success() {
            let url = response.url().clone();
            let body = response.bytes().await?;
            let error = String::from_utf8_lossy(&body).into_owned();
            Err(Error::Status { status, url, error })
        } else {
            Ok(Some(response.json().await?))
        }
    }

    pub async fn query_with_json<B, T>(&self, url: Url, body: Option<B>) -> Result<Option<T>, Error>
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

#[derive(Debug, Error, displaydoc::Display, From)]
pub enum Error {
    /// Transmitting a request or receiving the response failed: {0}
    Transport(reqwest::Error),
    /// Elastic Search failed, status={status}, url={url}, body={error}
    Status {
        status: StatusCode,
        url: Url,
        error: String,
    },
    /// Failed to serialize a requests or deserialize a response.
    Serialization(serde_json::Error),
    /// Given endpoint was not found: {0}
    EndpointNotFound(&'static str),
}

#[derive(derive_more::Into, Clone, Debug)]
pub struct SegmentableUrl(Url);

impl SegmentableUrl {
    pub fn with_segments(&self, segments: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        let mut new_url = self.0.clone();
        new_url.path_segments_mut()
            .unwrap(/* we made sure this can't happen */)
            .extend(segments);
        Self(new_url)
    }

    pub fn with_replaced_last_segment(&self, last_segment: &str) -> Self {
        let mut new_url = self.0.clone();
        let mut segments_mut = new_url.path_segments_mut()
            .unwrap(/* we made sure this can't happen */);
        segments_mut.pop().push(last_segment);
        drop(segments_mut);
        Self(new_url)
    }
}

impl TryFrom<Url> for SegmentableUrl {
    type Error = anyhow::Error;

    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.cannot_be_a_base() {
            Err(anyhow::anyhow!("non segmentable url"))
        } else {
            Ok(Self(url))
        }
    }
}

impl FromStr for SegmentableUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Url>()?.try_into()
    }
}
