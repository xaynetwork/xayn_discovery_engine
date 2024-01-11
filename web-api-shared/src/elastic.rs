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
    future::Future,
    hash::Hash,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use bytes::Bytes;
use derive_more::From;
use itertools::Itertools;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE},
    Body,
    Method,
    RequestBuilder,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{
    de::{self, DeserializeOwned},
    Deserialize,
    Deserializer,
    Serialize,
};
use serde_json::{json, Value};
use thiserror::Error;
use tracing::{error, warn};

use crate::{
    net::{ExponentialJitterRetryPolicy, ExponentialJitterRetryPolicyConfig},
    serde::{serde_duration_as_seconds, serialize_redacted, serialize_to_ndjson, JsonObject},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Config {
    pub url: String,
    pub user: String,
    #[serde(serialize_with = "serialize_redacted")]
    pub password: Secret<String>,
    pub index_name: String,

    /// Request timeout in seconds.
    #[serde(with = "serde_duration_as_seconds")]
    pub timeout: Duration,

    /// The retry policy for internal requests to elastic search.
    pub retry_policy: ExponentialJitterRetryPolicyConfig,

    pub default_request_per_second: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: "http://localhost:9200".into(),
            user: "elastic".into(),
            password: String::from("changeme").into(),
            index_name: "test_index".into(),
            timeout: Duration::from_secs(2),
            retry_policy: ExponentialJitterRetryPolicyConfig {
                max_retries: 3,
                step_size: Duration::from_millis(300),
                max_backoff: Duration::from_millis(1000),
            },
            default_request_per_second: 500,
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

/// A sanity check [`Client`] wrapper which indicates that the inner client doesn't necessary have the correct index set.
#[derive(Clone, Debug, From)]
pub struct ClientWithoutIndex(pub Client);

impl ClientWithoutIndex {
    pub fn new(config: Config) -> Result<Self, anyhow::Error> {
        Ok(Self(Client::new(config)?))
    }

    pub fn with_index(&self, index: impl AsRef<str>) -> Client {
        self.0.with_index(index)
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    auth: Arc<Auth>,
    url_to_index: Arc<SegmentableUrl>,
    client: reqwest::Client,
    retry_policy: ExponentialJitterRetryPolicyConfig,
    default_request_per_second: usize,
}

impl Client {
    pub fn new(config: Config) -> Result<Self, anyhow::Error> {
        let Config {
            url,
            user,
            password,
            index_name,
            timeout,
            retry_policy,
            default_request_per_second,
        } = config;
        Ok(Self {
            auth: Auth { user, password }.into(),
            url_to_index: url
                .parse::<SegmentableUrl>()?
                .with_segments([&index_name])
                .into(),
            client: reqwest::ClientBuilder::new().timeout(timeout).build()?,
            retry_policy,
            default_request_per_second,
        })
    }

    pub async fn retry<T, E, F>(
        &self,
        error_filter: impl Fn(&E) -> bool,
        code: impl FnMut() -> F,
    ) -> Result<T, E>
    where
        F: Future<Output = Result<T, E>>,
        E: Display,
    {
        ExponentialJitterRetryPolicy::new(self.retry_policy.clone())
            .with_retry_filter(error_filter)
            .retry(code)
            .await
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
            retry_policy: self.retry_policy.clone(),
            default_request_per_second: self.default_request_per_second,
        }
    }

    pub fn default_request_per_second(&self) -> usize {
        self.default_request_per_second
    }

    pub fn get_index(&self) -> &str {
        self.url_to_index.last_segment().unwrap(/*Client always has some index*/)
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
pub enum BulkInstruction<I> {
    Index {
        #[serde(rename = "_id")]
        id: I,
    },
    Create {
        #[serde(rename = "_id")]
        id: I,
    },
    Delete {
        #[serde(rename = "_id")]
        id: I,
    },
    Update {
        #[serde(rename = "_id")]
        id: I,
    },
}

#[derive(Debug, Deserialize)]
pub struct BulkItemResponse<I> {
    #[serde(rename = "_id")]
    pub id: I,
    pub status: u16,
    #[serde(default)]
    pub result: Option<String>,
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
    pub fn failed_documents(self, allow_not_found: bool, expected_result: &str) -> Vec<I>
    where
        I: Display + Debug,
    {
        self.errors
            .then(|| {
                self.items
                    .into_iter()
                    .filter_map(|response| {
                        if response.len() != 1 {
                            warn!(response=?response, "unexpected bulk response item");
                        }
                        // There was a bug with ES returning the wrong action (index for create),
                        // as we do check `result` we use that to find unexpected results.
                        let Some((action, response)) = response.into_iter().next() else {
                            //FIXME get id from zipping with query inputs, through also this should never happen so maybe don't bother
                            return None;
                        };
                        let result = response.result.as_deref().unwrap_or("none");
                        if !response.is_success_status(allow_not_found) {
                            error!(
                                document_id=%response.id,
                                error=%response.error,
                                "Elastic failed to {action} document.",
                            );
                            return Some(response.id);
                        }
                        if result != expected_result {
                            warn!(
                                expected=%expected_result,
                                got=%result,
                                "Mismatch in expected kind of result for bulk operation"
                            );
                        }
                        None
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
struct Hit<I> {
    #[serde(rename = "_id")]
    id: I,
    #[serde(rename = "_score")]
    score: f32,
}

#[derive(Debug, Deserialize)]
struct Hits<I> {
    hits: Vec<Hit<I>>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse<I> {
    hits: Hits<I>,
}

/// Deserializes from anything discarding any response.
///
/// Requires the `Deserializer` implementation to support `deserialize_any` and
/// might not work for cases where `visitor.visit_enum` is called.
///
/// This means it does work without restrictions for formats like `json`.
#[derive(Debug)]
pub struct SerdeDiscard;

impl<'de> Deserialize<'de> for SerdeDiscard {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Hint: a "discarding" serde type is more tricky than it might seem
        // 1. just returning `Ok(SerdeDiscard)` without calling the deserializer can fail
        //    in some edge cases due to the input which is supposed to be parsed not being
        //    parsed at all. Instead of being parsed and discarded.
        //    - this also can lead to a bunch of other unexpected behavior, e.g. we parsed a
        //      `_source` field even after we disabled `_source` inclusion and blindly returning
        //      `Ok(SerdeDiscard)` as a value in a map deserialized form json somehow didn't raise
        //      and error (even through it should have as only the value is discarded, which still means
        //      the fields itself is required if you don't also use `serde(default)`)
        // 2. not parsing the input can lead to two errors 1st the deserializer raising an
        //    error because something is wrong, 2nd we don't realize if the server has some
        //    major issues and send us partial bodies or similar
        // 3. to parse and then discard `deserialize_any` is required, or else the deserializer can't proceed at all
        // 4. if a derserializer is used which might call `visit_enum` (i.e. not json) it might still
        //    fail as there is no equivalent of deserialize_any for the enum variant
        // 5. Just using the serde derive on a empty `struct Foo { }` will only work with json object
        //    responses, but not arrays, string, etc.
        struct DiscardingVisitor;

        macro_rules! impl_simple_sink {
            ($($name:ident: $ty:ty),* $(,)?) => ($(
                fn $name<E>(self, _: $ty) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(SerdeDiscard)
                }
            )*);
        }

        impl<'de> de::Visitor<'de> for DiscardingVisitor {
            type Value = SerdeDiscard;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "nothing, anything should be fine")
            }

            impl_simple_sink! {
                visit_bool: bool,
                visit_i64: i64,
                visit_i128: i128,
                visit_u64: u64,
                visit_u128: u128,
                visit_f64: f64,
                visit_char: char,
                visit_str: &str,
                visit_string: String,
                visit_bytes: &[u8],
                visit_byte_buf: Vec<u8>,
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(SerdeDiscard)
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(Self)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(SerdeDiscard)
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(Self)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                while seq.next_element::<SerdeDiscard>()?.is_some() {}
                Ok(SerdeDiscard)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                while map.next_entry::<SerdeDiscard, SerdeDiscard>()?.is_some() {}
                Ok(SerdeDiscard)
            }

            fn visit_enum<A>(self, _: A) -> Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                //Hint: This can have the same issues as a deserialize `Ok(DiscardResponse)`,
                //      but we can't really do anything about it there is no `EnumAccess.variant_any`.
                Ok(SerdeDiscard)
            }
        }

        deserializer.deserialize_any(DiscardingVisitor)
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

        self.query_with_bytes::<BulkResponse<I>>(Method::POST, url, Some((headers, body.into())))
            .await
    }

    pub async fn search_request<F, I, E>(
        &self,
        mut body: JsonObject,
        parse_id: F,
    ) -> Result<ScoreMap<I>, E>
    where
        F: Fn(String) -> Result<I, E>,
        I: Eq + Hash,
        E: From<Error>,
    {
        if body.get("size") == Some(&json!(0)) {
            return Ok(HashMap::new());
        }
        body.insert("_source".into(), json!(false));
        body.insert("track_total_hits".into(), json!(false));

        let response = self
            .query_with_json::<_, SearchResponse<String>>(
                Method::POST,
                self.create_url(["_search"], None),
                Some(body),
            )
            .await?;

        response
            .hits
            .hits
            .into_iter()
            .map(|hit| Ok((parse_id(hit.id)?, hit.score)))
            .try_collect()
    }

    pub async fn query_with_bytes<T>(
        &self,
        method: Method,
        url: Url,
        post_data: Option<(HeaderMap<HeaderValue>, Bytes)>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.retry(
            |err| matches!(err, Error::Transport(_)),
            || async {
                let method = method.clone();
                let url = url.clone();
                let post_data = post_data.clone();
                self.query_with_bytes_without_retrying(method, url, post_data)
                    .await
            },
        )
        .await
    }

    async fn query_with_bytes_without_retrying<B, T>(
        &self,
        method: Method,
        url: Url,
        post_data: Option<(HeaderMap<HeaderValue>, B)>,
    ) -> Result<T, Error>
    where
        B: Into<Body>,
        T: DeserializeOwned,
    {
        let path = url.path().to_owned();
        let mut request_builder = self.client.request(method, url);
        if let Some((headers, body)) = post_data {
            request_builder = request_builder.headers(headers).body(body)
        }

        let response = self.auth.apply_to(request_builder).send().await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            Err(Error::ResourceNotFound(path))
        } else if !status.is_success() {
            let url = response.url().clone();
            let body = response.bytes().await?;
            let error = String::from_utf8_lossy(&body).into_owned();
            Err(Error::Status { status, url, error })
        } else {
            let body = response.bytes().await?;
            Ok(serde_json::from_slice(&body)?)
        }
    }

    pub async fn query_with_json<B, T>(
        &self,
        method: Method,
        url: Url,
        body: Option<B>,
    ) -> Result<T, Error>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let post_data = body
            .map(|json| -> Result<_, Error> {
                let mut headers = HeaderMap::new();
                headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                let body = serde_json::to_vec(&json)?;
                Ok((headers, body.into()))
            })
            .transpose()?;

        self.query_with_bytes(method, url, post_data).await
    }
}

pub type ScoreMap<Id> = HashMap<Id, f32>;

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
    /// Failed to serialize a requests or deserialize a response: {0}
    Serialization(serde_json::Error),
    /// Given resource was not found: {0}
    ResourceNotFound(String),
}

pub trait NotFoundAsOptionExt<T> {
    fn not_found_as_option(self) -> Result<Option<T>, Error>;
}

impl<T> NotFoundAsOptionExt<T> for Result<T, Error> {
    fn not_found_as_option(self) -> Result<Option<T>, Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(Error::ResourceNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }
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

    pub fn last_segment(&self) -> Option<&str> {
        self.0.path_segments()
            .unwrap(/* we made sure this can't happen */)
            .last()
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
