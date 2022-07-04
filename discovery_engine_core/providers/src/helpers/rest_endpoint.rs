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

use std::{sync::Arc, time::Duration};

use once_cell::sync::OnceCell;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use url::Url;

use crate::Error;

/// Preferably all endpoints should share the same [`Client`] instance.
static SHARED_CLIENT: OnceCell<Arc<Client>> = OnceCell::new();

/// A simple abstraction over a single endpoint.
pub struct RestEndpoint {
    client: Arc<Client>,
    url: Url,
    auth_token: String,
    timeout: Duration,
    get_as_post: bool,
}

impl RestEndpoint {
    /// Create a `RestEndpoint` instance with a default timeout.
    pub fn new(url: Url, auth_token: String) -> Self {
        let client = SHARED_CLIENT
            .get_or_init(|| {
                // Note: If we need to use a ClientBuilder we should pass the `Arc<Client>` as
                //       argument to `new` instead.
                Arc::new(Client::new())
            })
            .clone();

        Self {
            client,
            url,
            auth_token,
            timeout: Duration::from_millis(3500),
            get_as_post: false,
        }
    }

    /// Configures the timeout.
    ///
    /// The timeout defaults to 3.5s.
    #[must_use = "dropped changed client"]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configures if we should use POST for GET requests.
    ///
    /// This is sometimes needed when some part of the serve
    /// pipeline puts to strict limits on the length of the
    /// path/query.
    ///
    /// It's semantically still a GET request.
    #[must_use = "dropped changed client"]
    pub fn with_get_as_post(mut self, get_as_post: bool) -> Self {
        self.get_as_post = get_as_post;
        self
    }

    /// Return a reference to the Url of the endpoint.
    pub fn url(&self) -> &Url {
        &self.url
    }

    pub async fn get_request<
        D: DeserializeOwned + Send,
        FN: FnOnce(&mut dyn FnMut(&str, String)) + Send,
    >(
        &self,
        setup_query_params: FN,
    ) -> Result<D, Error> {
        let mut url = self.url.clone();

        let query_builder = if self.get_as_post {
            let mut query = Map::new();
            setup_query_params(&mut |key, value| {
                query.insert(key.into(), Value::String(value));
            });
            let body = serde_json::to_vec(&Value::Object(query)).map_err(Error::Encoding)?;
            self.client.post(url).body(body)
        } else {
            let mut query_mut = url.query_pairs_mut();
            setup_query_params(&mut |key, value| {
                query_mut.append_pair(key, &value);
            });
            drop(query_mut);
            self.client.get(url)
        };

        let response = query_builder
            .timeout(self.timeout)
            .bearer_auth(&self.auth_token)
            .send()
            .await
            .map_err(Error::RequestExecution)?
            .error_for_status()
            .map_err(Error::StatusCode)?;

        let raw_response = response.bytes().await.map_err(Error::Fetching)?;
        let deserializer = &mut serde_json::Deserializer::from_slice(&raw_response);
        serde_path_to_error::deserialize(deserializer)
            .map_err(|error| Error::DecodingAtPath(error.path().to_string(), error))
    }
}
