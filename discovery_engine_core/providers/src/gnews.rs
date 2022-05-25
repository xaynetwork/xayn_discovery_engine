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

//! Client to get new documents.

use async_trait::async_trait;
use derive_more::From;
use itertools::Itertools;
use url::{form_urlencoded, Url, UrlQuery};

use crate::{
    rest::SimpleEndpoint,
    Article,
    CommonQueryParts,
    Error,
    HeadlinesProvider,
    HeadlinesQuery,
    NewsProvider,
    NewsQuery,
};

use self::models::Response;

mod models;

/// Gnews based implementation of a `NewsProvider`.
#[derive(From)]
pub struct NewsProviderImpl(SimpleEndpoint);

impl NewsProviderImpl {
    /// Create a new provider instance.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(SimpleEndpoint::new(endpoint_url, auth_token))
    }
}

#[async_trait]
impl NewsProvider for NewsProviderImpl {
    async fn query_news(&self, request: &NewsQuery<'_>) -> Result<Vec<Article>, Error> {
        let response: Response = self
            .0
            .fetch(|mut query| {
                query.append_pair("sortby", "relevance");
                common_query_parts_helper(&request.common, &mut query);
                Ok(())
            })
            .await?;

        Ok(response.articles.into_iter().map_into().collect())
    }
}

/// Gnews based implementation of a `HeadlinesProvider`.
#[derive(From)]
pub struct HeadlineProviderImpl(SimpleEndpoint);

impl HeadlineProviderImpl {
    /// Create a new provider instance.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(SimpleEndpoint::new(endpoint_url, auth_token))
    }
}

#[async_trait]
impl HeadlinesProvider for HeadlineProviderImpl {
    async fn query_headlines(&self, request: &HeadlinesQuery<'_>) -> Result<Vec<Article>, Error> {
        let response: Response = self
            .0
            .fetch(|mut query| {
                common_query_parts_helper(&request.common, &mut query);
                if let Some(topic) = &request.common.topic {
                    query.append_pair("topic", topic);
                }
                Ok(())
            })
            .await?;

        Ok(response.articles.into_iter().map_into().collect())
    }
}

fn common_query_parts_helper(
    common: &CommonQueryParts<'_>,
    query: &mut form_urlencoded::Serializer<'_, UrlQuery<'_>>,
) {
    query
        .append_pair("max", &common.page_size.to_string())
        .append_pair("page", &common.page.to_string());

    if let Some(filter) = &common.filter {
        query.append_pair("q", &filter.build());
    }

    if let Some(market) = &common.market {
        query
            .append_pair("lang", &market.lang_code)
            .append_pair("country", &market.country_code.to_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use crate::{Filter, Market};

    use super::*;

    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    #[tokio::test]
    async fn test_simple_news_query() {
        let mock_server = MockServer::start().await;
        let provider = NewsProviderImpl::new(
            Url::parse(&format!("{}/v2/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/gnews/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/v2/search-news"))
            .and(query_param("q", "\"Climate change\""))
            .and(query_param("sortby", "relevance"))
            .and(query_param("lang", "en"))
            .and(query_param("country", "au"))
            .and(query_param("max", "2"))
            .and(query_param("page", "1"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market {
            lang_code: "en".to_string(),
            country_code: "AU".to_string(),
        };
        let filter = &Filter::default().add_keyword("Climate change");

        let query = NewsQuery {
            common: CommonQueryParts {
                market: Some(market),
                page_size: 2,
                page: 1,
                excluded_sources: &[],
                trusted_sources: &[],
                filter: Some(filter),
                topic: None,
            },
            from: None,
        };

        let docs = provider.query_news(&query).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(
            doc.title,
            "[WATCH] #StoryOfTheNation: Environment, climate change, and the 2022 polls"
        );
    }
}
