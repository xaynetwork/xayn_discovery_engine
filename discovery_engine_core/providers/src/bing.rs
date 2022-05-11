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

use crate::{seal::Seal, Client, Error, Market, NewscatcherQuery};

const URL_SUFFIX: &str = "_tt";

/// Parameters for fetching trending news topics.
pub struct TrendingQuery<'a> {
    /// Market to fetch results from.
    pub market: &'a Market,
}

//TODO[TY-2810] somehow we ended up with bing queries implementing the newscatcher query trait,
//              but passing them to `query_newscatcher` wouldn't work. So this needs to change.
impl NewscatcherQuery for TrendingQuery<'_> {
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrlBase(None))?
            .push(URL_SUFFIX);

        let query = &mut url.query_pairs_mut();

        let lang = &self.market.lang_code;
        let country = &self.market.country_code;
        query.append_pair("mkt", &format!("{}-{}", lang, country));

        Ok(())
    }
}

impl Seal for TrendingQuery<'_> {}

impl Client {
    /// Run query for fetching trending topics from Bing.
    pub async fn query_trending(
        &self,
        query: &impl NewscatcherQuery,
    ) -> Result<Vec<TrendingTopic>, Error> {
        self.query_bing(query).await.map(|trending| trending.value)
    }

    /// Run a query against Bing.
    pub async fn query_bing(&self, query: &impl NewscatcherQuery) -> Result<Response, Error> {
        let mut url =
            Url::parse(&self.newscatcher.url).map_err(|e| Error::InvalidUrlBase(Some(e)))?;
        query.setup_url(&mut url)?;

        let response = self
            .newscatcher
            .client
            .get(url)
            .timeout(self.newscatcher.timeout)
            .bearer_auth(&self.newscatcher.token)
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
    pub(crate) name: String,

    /// Search query that returns this topic.
    #[serde(default)]
    pub(crate) query: SearchQuery,

    /// Link to a related image.
    #[serde(default)]
    pub(crate) image: Image,
}

/// Search query returning a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct SearchQuery {
    /// Query text.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) text: String,
}

/// Image relating to a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
pub(crate) struct Image {
    /// URL to the image.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) url: String,
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

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use super::*;

    #[test]
    // in order to make sure that our API clients don't throw errors if some trending topics
    // are malformed (missing fields, null fields) we are very liberal in what we accept
    // as trending topics
    fn test_deserialize_topic_where_all_fields_should_fall_back_to_default() {
        let _topic: TrendingTopic = serde_json::from_str("{}").unwrap();
    }

    #[tokio::test]
    async fn test_trending() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/trending-topics.json"
        ));

        Mock::given(method("GET"))
            .and(path("/_tt"))
            .and(query_param("mkt", "en-US"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market {
            lang_code: "en".to_string(),
            country_code: "US".to_string(),
        };
        let params = TrendingQuery { market };

        let topics = client.query_trending(&params).await.unwrap();
        assert_eq!(topics.len(), 25);

        let topic = topics.get(0).unwrap();
        let expected = TrendingTopic {
            name: "40% out of stock".to_string(),
            query: SearchQuery {
                text: "Baby formula shortage 40".to_string(),
            },
            image: Image {
                url: "https://example.com/image.jpg".to_string(),
            },
        };

        assert_eq!(format!("{:?}", topic), format!("{:?}", expected));
    }
}
