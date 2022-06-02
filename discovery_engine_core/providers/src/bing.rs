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

use crate::{rest::Endpoint, Error, Market};

/// Parameters for fetching trending news topics.
pub struct TrendingQuery<'a> {
    /// Market to fetch results from.
    pub market: &'a Market,
}

/// Bing implementation for tending topics.
pub struct TrendingTopicsProvider(Endpoint);

impl TrendingTopicsProvider {
    /// Create a new provider.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(Endpoint::new(endpoint_url, auth_token))
    }

    /// Creates a provider from given endpoint.
    pub fn from_endpoint(endpoint: Endpoint) -> TrendingTopicsProvider {
        Self(endpoint)
    }

    /// Run query for fetching trending topics from Bing.
    //Note: If we ever have potentially multiple providers we can make this a trait like for e.g. latest-headlines
    pub async fn query_trending_topics(
        &self,
        request: &TrendingQuery<'_>,
    ) -> Result<Vec<TrendingTopic>, Error> {
        self.0
            .fetch::<Response, _>(|mut query| {
                let lang = &request.market.lang_code;
                let country = &request.market.country_code;
                query.append_pair("mkt", &format!("{}-{}", lang, country));
            })
            .await
            .map(|response| response.value)
    }
}

/// Query response from Bing API.
#[derive(Deserialize, Debug)]
struct Response {
    /// Main response content.
    #[serde(default)]
    value: Vec<TrendingTopic>,
}

/// Trending topic.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrendingTopic {
    /// Title of the trending topic.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,

    /// Search query that returns this topic.
    #[serde(default)]
    pub query: SearchQuery,

    /// Link to a related image.
    #[serde(default)]
    pub image: Image,
}

/// Search query returning a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SearchQuery {
    /// Query text.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub text: String,
}

/// Image relating to a trending topic.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Image {
    /// URL to the image.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub url: String,
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
        let mut url = Url::parse(&mock_server.uri()).unwrap();
        url.path_segments_mut().unwrap().push("_tt");
        let client = TrendingTopicsProvider::new(url, "test-token".into());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!(
                "../test-fixtures/bing/trending-topics.json"
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

        let topics = client.query_trending_topics(&params).await.unwrap();
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
