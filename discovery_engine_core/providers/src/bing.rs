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

//! Client to retrieve trending topics.

use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

use crate::{seal::Seal, Client, Error, Market, Query};

const URL_SUFFIX: &str = "_tt";

/// Parameters for fetching trending news topics.
pub struct TrendingQuery<'a> {
    /// Market to fetch results from.
    pub market: &'a Market,
}

impl Query for TrendingQuery<'_> {
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrlBase(None))?
            .push(URL_SUFFIX);

        let query = &mut url.query_pairs_mut();

        let country = &self.market.country_code;
        let lang = &self.market.lang_code;
        query.append_pair("mkt", &format!("{}-{}", lang, country));

        Ok(())
    }
}

impl Seal for TrendingQuery<'_> {}

impl Client {
    /// Run query for fetching trending topics from Bing.
    pub async fn query_trending(&self, query: &impl Query) -> Result<Vec<TrendingTopic>, Error> {
        self.query_bing(query).await.map(|trending| trending.value)
    }

    /// Run a query against Bing.
    pub async fn query_bing(&self, query: &impl Query) -> Result<Response, Error> {
        let mut url = Url::parse(&self.url).map_err(|e| Error::InvalidUrlBase(Some(e)))?;
        query.setup_url(&mut url)?;

        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .bearer_auth(&self.token)
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

/// Query response from Bing API.
#[derive(Deserialize, Debug)]
pub struct Response {
    /// Main response content.
    #[serde(default)]
    value: Vec<TrendingTopic>,
}

/// Trending topic.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrendingTopic {
    /// Title of the trending topic.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    name: String,

    /// Search query that returns this topic.
    #[serde(default)]
    query: SearchQuery,

    /// Link to a related image.
    #[serde(default)]
    image: Image,
}

/// Search query returning a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
struct SearchQuery {
    /// Query text.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    text: String,
}

/// Image relating to a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
struct Image {
    /// URL to the image.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    url: String,
}

// TODO relocate
// Taken from https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
