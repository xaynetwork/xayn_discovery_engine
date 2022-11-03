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

use std::time::Duration;

use url::Url;

use crate::{error::Error, utils::rest_endpoint::RestEndpoint};

/// The provider configurations.
#[must_use]
pub struct Config {
    /// The url to the provider API.
    pub(crate) url: Url,
    /// The auth token/key to the provider API.
    pub(crate) token: String,
    /// The timeout of the provider API. Defaults to 3500ms.
    pub(crate) timeout: Duration,
    /// The number of retries in case of a timeout. Defaults to 0.
    pub(crate) retry: usize,
    /// Use POST for GET requests.
    ///
    /// For servers with limited query length. It's semantically still a GET request.
    pub(crate) get_as_post: bool,
}

impl Config {
    pub(crate) const SEARCH: &str = "/newscatcher/v2/search";
    pub(crate) const SIMILAR_SEARCH: &str = "/_mlt";
    pub(crate) const HEADLINES: &str = "/newscatcher/v2/latest_headlines";
    pub(crate) const TRUSTED_HEADLINES: &str = "/newscatcher/v2/trusted-sources";

    pub(crate) fn new(
        base: &str,
        route: &str,
        token: impl Into<String>,
        get_as_post: bool,
    ) -> Result<Self, Error> {
        Ok(Self {
            url: Url::parse(base)
                .and_then(|base| base.join(route))
                .map_err(Error::MalformedUrlInConfig)?,
            token: token.into(),
            timeout: Duration::from_millis(3500),
            retry: 0,
            get_as_post,
        })
    }

    pub fn search(
        base: &str,
        route: Option<&str>,
        token: impl Into<String>,
    ) -> Result<Self, Error> {
        Self::new(base, route.unwrap_or(Self::SEARCH), token, true)
    }

    pub fn similar_search(
        base: &str,
        route: Option<&str>,
        token: impl Into<String>,
    ) -> Result<Self, Error> {
        Self::new(base, route.unwrap_or(Self::SIMILAR_SEARCH), token, true)
    }

    pub fn headlines(
        base: &str,
        route: Option<&str>,
        token: impl Into<String>,
    ) -> Result<Self, Error> {
        Self::new(base, route.unwrap_or(Self::HEADLINES), token, true)
    }

    pub fn trusted_headlines(
        base: &str,
        route: Option<&str>,
        token: impl Into<String>,
    ) -> Result<Self, Error> {
        Self::new(base, route.unwrap_or(Self::TRUSTED_HEADLINES), token, true)
    }

    pub fn with_timeout(mut self, millis: u64) -> Self {
        self.timeout = Duration::from_millis(millis);
        self
    }

    pub fn with_retry(mut self, retry: usize) -> Self {
        self.retry = retry;
        self
    }

    pub fn build(self) -> RestEndpoint {
        RestEndpoint::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let base = "https://api.example.com";
        let token = "test-token";
        let _config = Config::search(base, None, token).unwrap();
        let _config = Config::similar_search(base, None, token).unwrap();
        let _config = Config::headlines(base, None, token).unwrap();
        let _config = Config::trusted_headlines(base, None, token).unwrap();
    }
}
