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

use crate::{error::Error, helpers::rest_endpoint::RestEndpoint};

/// The provider configurations.
#[must_use]
pub struct Config {
    /// The url to the provider API.
    pub(crate) url: Url,
    /// The auth token/key to the provider API.
    pub(crate) token: String,
    /// The timeout of the provider API.
    pub(crate) timeout: Duration,
    /// The number of retries in case of a timeout.
    pub(crate) retry: usize,
    /// Use POST for GET requests.
    ///
    /// For servers with limited query length. It's semantically still a GET request.
    pub(crate) get_as_post: bool,
}

impl Config {
    pub(crate) const NEWS: &'static str = "/newscatcher/v2/search";
    pub(crate) const SIMILAR_NEWS: &'static str = "/_mlt";
    pub(crate) const HEADLINES: &'static str = "/newscatcher/v2/latest_headlines";
    pub(crate) const TRUSTED_HEADLINES: &'static str = "/newscatcher/v2/trusted-sources";
    pub(crate) const TRENDING_TOPICS: &'static str = "/bing/v1/trending-topics";

    pub(crate) fn new(
        base: &str,
        route: &'static str,
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

    pub fn news(base: &str, token: impl Into<String>) -> Result<Self, Error> {
        Self::new(base, Self::NEWS, token, true)
    }

    pub fn similar_news(base: &str, token: impl Into<String>) -> Result<Self, Error> {
        Self::new(base, Self::SIMILAR_NEWS, token, true)
    }

    pub fn headlines(base: &str, token: impl Into<String>) -> Result<Self, Error> {
        Self::new(base, Self::HEADLINES, token, true)
    }

    pub fn trusted_headlines(base: &str, token: impl Into<String>) -> Result<Self, Error> {
        Self::new(base, Self::TRUSTED_HEADLINES, token, true)
    }

    pub fn trending_topics(base: &str, token: impl Into<String>) -> Result<Self, Error> {
        Self::new(base, Self::TRENDING_TOPICS, token, false)
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
        let _config = Config::news(base, token).unwrap();
        let _config = Config::similar_news(base, token).unwrap();
        let _config = Config::headlines(base, token).unwrap();
        let _config = Config::trusted_headlines(base, token).unwrap();
        let _config = Config::trending_topics(base, token).unwrap();
    }
}
