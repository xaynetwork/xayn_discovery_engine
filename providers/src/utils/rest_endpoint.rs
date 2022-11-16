// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public L  icense for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::sync::Arc;

use once_cell::sync::OnceCell;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{config::Config, error::Error};

/// Preferably all endpoints should share the same [`Client`] instance.
static SHARED_CLIENT: OnceCell<Arc<Client>> = OnceCell::new();

/// A simple abstraction over a single endpoint.
pub struct RestEndpoint {
    client: Arc<Client>,
    pub(crate) config: Config,
}

impl RestEndpoint {
    /// Create a `RestEndpoint` instance with a default timeout.
    pub fn new(config: Config) -> Self {
        let client = SHARED_CLIENT
            .get_or_init(|| {
                // Note: If we need to use a ClientBuilder we should pass the `Arc<Client>` as
                //       argument to `new` instead.
                Arc::new(Client::new())
            })
            .clone();

        Self { client, config }
    }

    pub async fn get_request<F, D>(&self, setup_query_params: F) -> Result<D, Error>
    where
        F: Fn(&mut dyn FnMut(&str, String)) + Send + Sync,
        D: DeserializeOwned + Send,
    {
        let request_builder = || {
            let mut url = self.config.url.clone();
            if self.config.get_as_post {
                let mut query = Map::new();
                setup_query_params(&mut |key, value| {
                    query.insert(key.into(), Value::String(value));
                });
                self.client.post(url).json(&Value::Object(query))
            } else {
                let mut query_mut = url.query_pairs_mut();
                setup_query_params(&mut |key, value| {
                    query_mut.append_pair(key, &value);
                });
                drop(query_mut);
                self.client.get(url)
            }
            .timeout(self.config.timeout)
            .bearer_auth(&self.config.token)
        };

        let mut retry = 0;
        let response = loop {
            match request_builder().send().await {
                Err(error) if error.is_timeout() && retry < self.config.retry => {
                    retry += 1;
                    continue;
                }
                result => {
                    break result
                        .map_err(Error::RequestExecution)?
                        .error_for_status()
                        .map_err(Error::StatusCode)?;
                }
            }
        };

        let raw_response = response.bytes().await.map_err(Error::Fetching)?;
        let deserializer = &mut serde_json::Deserializer::from_slice(&raw_response);
        serde_path_to_error::deserialize(deserializer)
            .map_err(|error| Error::DecodingAtPath(error.path().to_string(), error))
    }
}
