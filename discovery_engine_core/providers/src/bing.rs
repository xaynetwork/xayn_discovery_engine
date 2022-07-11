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

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use url::Url;

use crate::{models::TrendingTopicsQuery, Error, RestEndpoint, TrendingTopicsProvider};

pub struct BingTrendingTopicsProvider {
    endpoint: RestEndpoint,
}

impl BingTrendingTopicsProvider {
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self {
            endpoint: RestEndpoint::new(endpoint_url, auth_token),
        }
    }

    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn TrendingTopicsProvider> {
        Arc::new(Self { endpoint })
    }
}

#[async_trait]
impl TrendingTopicsProvider for BingTrendingTopicsProvider {
    /// Run query for fetching trending topics from Bing.
    async fn query_trending_topics(
        &self,
        request: &TrendingTopicsQuery<'_>,
    ) -> Result<Vec<TrendingTopic>, Error> {
        let response = self
            .endpoint
            .get_request::<Response, _>(|query_append| {
                let lang = &request.market.lang_code;
                let country = &request.market.country_code;
                query_append("mkt", format!("{}-{}", lang, country));
            })
            .await?;

        Ok(response.value)
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
    use crate::{Market, TrendingTopicsQuery};
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
        let endpoint_url = Url::parse(&mock_server.uri()).unwrap();
        let provider = BingTrendingTopicsProvider::new(endpoint_url, "test-token".to_string());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/trending-topics.json"));

        Mock::given(method("GET"))
            .and(path("/"))
            .and(query_param("mkt", "en-US"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market::new("en", "US");
        let params = TrendingTopicsQuery { market };

        let topics = provider.query_trending_topics(&params).await.unwrap();
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
