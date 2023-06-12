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

use std::{sync::Arc, time::Duration};

use anyhow::bail;
use derive_more::From;
use displaydoc::Display;
use reqwest::{
    header::{HeaderName, HeaderValue},
    Method,
    StatusCode,
    Url,
};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

use crate::{
    serde::{serde_duration_as_seconds, serialize_redacted},
    url::SegmentableUrl,
    SetupError,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub environment: String,
    pub project: String,
    pub index: String,
    pub namespace: String,
    #[serde(serialize_with = "serialize_redacted")]
    pub api_key: Secret<String>,
    #[serde(with = "serde_duration_as_seconds")]
    pub timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            environment: "asia-southeast1-gcp-free".into(),
            project: "58855b0".into(),
            index: "test".into(),
            namespace: "test".into(),
            api_key: "change-me".to_string().into(),
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Clone, Debug)]
struct Tenant {
    environment: String,
    project: String,
    index: String,
    namespace: String,
}

#[derive(Clone, Debug)]
pub struct Client {
    tenant: Arc<Tenant>,
    api_key: Arc<Secret<String>>,
    client: reqwest::Client,
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// Transmitting a request or receiving the response failed: {0}
    Transport(reqwest::Error),
    /// Pinecone failed, status={status}, url={url}, body={error}
    Status {
        status: StatusCode,
        url: Url,
        error: String,
    },
    /// Failed to serialize a requests or deserialize a response.
    Serialization(serde_json::Error),
}

impl Client {
    pub fn new(config: Config) -> Result<Self, SetupError> {
        segmentable_url(&config.index, &config.project, &config.environment)?;
        api_key_header(&config.api_key)?;

        let tenant = Tenant {
            environment: config.environment,
            project: config.project,
            index: config.index,
            namespace: config.namespace,
        }
        .into();
        let api_key = config.api_key.into();
        let client = reqwest::ClientBuilder::new()
            .timeout(config.timeout)
            .build()?;

        Ok(Self {
            tenant,
            api_key,
            client,
        })
    }

    pub fn with_index_and_namespace(
        &self,
        index: impl AsRef<str>,
        namespace: impl AsRef<str>,
    ) -> Result<Self, SetupError> {
        let index = index.as_ref();
        segmentable_url(index, &self.tenant.project, &self.tenant.environment)?;
        let tenant = Tenant {
            environment: self.tenant.environment.clone(),
            project: self.tenant.project.clone(),
            index: index.into(),
            namespace: namespace.as_ref().into(),
        }
        .into();

        Ok(Self {
            tenant,
            api_key: self.api_key.clone(),
            client: self.client.clone(),
        })
    }

    pub async fn request<'a, R>(
        &self,
        method: Method,
        segments: impl IntoIterator<Item = impl AsRef<str>>,
        params: impl IntoIterator<Item = (impl AsRef<str>, Option<impl AsRef<str>>)>,
        body: Option<impl Serialize>,
        with_namespace: bool,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned,
    {
        let mut url = segmentable_url(
            &self.tenant.index,
            &self.tenant.project,
            &self.tenant.environment,
        )
        .unwrap(/* checked in constructor */)
        .with_segments(segments)
        .with_params(params);
        if with_namespace && body.is_none() {
            url = url.with_params([("namespace", Some(&self.tenant.namespace))]);
        }
        let (api_key_name, api_key_value) =
            api_key_header(&self.api_key).unwrap(/* checked in constructor */);

        let mut request = self
            .client
            .request(method, url.into_inner())
            .header(api_key_name, api_key_value);
        if let Some(body) = body {
            if with_namespace {
                let Value::Object(mut body) = serde_json::to_value(body)? else {
                    unreachable!(/* body is a json object */);
                };
                body.insert("namespace".to_string(), json!(&self.tenant.namespace));
                request = request.json(&body);
            } else {
                request = request.json(&body);
            }
        }

        let response = request.send().await?;
        if response.status().is_success() {
            response.json().await.map_err(Into::into)
        } else {
            let status = response.status();
            let url = response.url().clone();
            let error = String::from_utf8_lossy(&response.bytes().await?).into_owned();

            Err((status, url, error).into())
        }
    }
}

fn api_key_header(api_key: &Secret<String>) -> Result<(HeaderName, HeaderValue), SetupError> {
    let name = HeaderName::from_static("api-key");
    let Ok(mut value) = HeaderValue::from_str(api_key.expose_secret()) else {
        bail!("api key must only contain visible ascii characters");
    };
    value.set_sensitive(true);

    Ok((name, value))
}

fn segmentable_url(
    index: &str,
    project: &str,
    environment: &str,
) -> Result<SegmentableUrl, SetupError> {
    format!("https://{index}-{project}.svc.{environment}.pinecone.io").parse::<SegmentableUrl>()
}
