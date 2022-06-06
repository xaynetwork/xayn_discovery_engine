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

use std::sync::Arc;

use async_trait::async_trait;
use url::{form_urlencoded, Url, UrlQuery};

use crate::{
    rest::Endpoint,
    Article,
    CommonQueryParts,
    Error,
    HeadlinesProvider,
    HeadlinesQuery,
    Market,
    NewsProvider,
    NewsQuery,
};

use self::models::Response;

mod models;

/// Gnews based implementation of a `NewsProvider`.
pub struct NewsProviderImpl(Endpoint);

impl NewsProviderImpl {
    /// Create a new provider instance.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(Endpoint::new(endpoint_url, auth_token))
    }

    /// Creates a `Arc<dyn NewsProvider>` from given endpoint.
    pub fn from_endpoint(endpoint: Endpoint) -> Arc<dyn NewsProvider> {
        Arc::new(Self(endpoint))
    }
}

#[async_trait]
impl NewsProvider for NewsProviderImpl {
    async fn query_news(&self, request: &NewsQuery<'_>) -> Result<Vec<Article>, Error> {
        self.0
            .fetch::<Response, _>(|mut query| {
                query.append_pair("sortby", "relevance")
                    .append_pair("q", &request.filter.build())
                    .append_pair("in", "title,description,content");

                append_common_query_parts(&mut query, &request.common);
                append_market(&mut query, request.market);
            })
            .await
            .map(|response| {
                response
                    .articles
                    .into_iter()
                    .map(|article| article.into_generic_article(request.market.clone(), "".into()))
                    .collect()
            })
    }
}

/// Gnews based implementation of a `HeadlinesProvider`.
pub struct HeadlinesProviderImpl(Endpoint);

impl HeadlinesProviderImpl {
    /// Create a new provider instance.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(Endpoint::new(endpoint_url, auth_token))
    }

    /// Creates a `Arc<dyn HeadlineProvider>` from given endpoint.
    pub fn from_endpoint(endpoint: Endpoint) -> Arc<dyn HeadlinesProvider> {
        Arc::new(Self(endpoint))
    }
}

#[async_trait]
impl HeadlinesProvider for HeadlinesProviderImpl {
    async fn query_headlines(&self, request: &HeadlinesQuery<'_>) -> Result<Vec<Article>, Error> {
        self.0
            .fetch::<Response, _>(|mut query| {
                append_common_query_parts(&mut query, &request.common);
                append_market(&mut query, request.market);

                if let Some(topic) = &request.topic {
                    query.append_pair("topic", topic);
                }
            })
            .await
            .map(|response| {
                response
                    .articles
                    .into_iter()
                    .map(|article| {
                        article.into_generic_article(
                            request.market.clone(),
                            request.topic.unwrap_or("").into(),
                        )
                    })
                    .collect()
            })
    }
}

fn append_common_query_parts(
    query: &mut form_urlencoded::Serializer<'_, UrlQuery<'_>>,
    common: &CommonQueryParts<'_>,
) {
    query
        .append_pair("max", &common.page_size.to_string())
        .append_pair("page", &common.page.to_string());
}

fn append_market(query: &mut form_urlencoded::Serializer<'_, UrlQuery<'_>>, market: &Market) {
    query
        .append_pair("lang", &market.lang_code)
        .append_pair("country", &market.country_code.to_lowercase());
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
            .and(query_param("q", "(Climate change)"))
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
                page_size: 2,
                page: 1,
                excluded_sources: &[],
            },
            market,
            filter,
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
