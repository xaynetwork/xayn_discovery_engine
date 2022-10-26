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

use displaydoc::Display;
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{impl_application_error, utils::serialize_redacted};

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
    #[allow(dead_code)]
    config: Config,
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl ElasticSearchClient {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn query_elastic_search<B, T>(
        &self,
        route: &str,
        body: Option<B>,
    ) -> Result<Option<T>, ElasticSearchError>
    where
        B: Serialize,
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
        .basic_auth(
            &self.config.user,
            Some(self.config.password.expose_secret()),
        )
        .send()
        .await
        .map_err(InternalSeriveError)?
        .error_for_status()
        //TODO handle 404
        .map_err(InternalSeriveError)?
        .json()
        .await
        .map_err(InternalSeriveError)
    }
}
