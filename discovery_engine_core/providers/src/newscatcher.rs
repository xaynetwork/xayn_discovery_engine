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
    rest::Endpoint,
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

/// Newscatcher based implementation of a `NewsProvider`.
#[derive(From)]
pub struct NewsProviderImpl(Endpoint);

impl NewsProviderImpl {
    /// Create a new provider.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(Endpoint::new(endpoint_url, auth_token))
    }
}

#[async_trait]
impl NewsProvider for NewsProviderImpl {
    async fn query_news(&self, request: &NewsQuery<'_>) -> Result<Vec<Article>, Error> {
        self.0
            .fetch::<Response, _>(|mut query| {
                query.append_pair("sort_by", "relevancy");

                append_common_query_parts(&request.common, &mut query);
                query.append_pair("q", &request.filter.build());

                if let Some(from) = &request.from {
                    query.append_pair("from", from);
                }
            })
            .await
            .map(|response| response.articles.into_iter().map_into().collect())
    }
}

/// Newscatcher based implementation of a `HeadlinesProvider`.
#[derive(From)]
pub struct HeadlinesProviderImpl(Endpoint);

impl HeadlinesProviderImpl {
    /// Create a new provider.
    pub fn new(endpoint_url: Url, auth_token: String) -> Self {
        Self(Endpoint::new(endpoint_url, auth_token))
    }
}

#[async_trait]
impl HeadlinesProvider for HeadlinesProviderImpl {
    async fn query_headlines(&self, request: &HeadlinesQuery<'_>) -> Result<Vec<Article>, Error> {
        self.0
            .fetch::<Response, _>(|mut query| {
                append_common_query_parts(&request.common, &mut query);
                if let Some(topic) = &request.topic {
                    query.append_pair("topic", topic);
                }
                if let Some(when) = &request.when {
                    query.append_pair("when", when);
                }
            })
            .await
            .map(|response| response.articles.into_iter().map_into().collect())
    }
}

fn append_common_query_parts(
    common: &CommonQueryParts<'_>,
    query: &mut form_urlencoded::Serializer<'_, UrlQuery<'_>>,
) {
    if let Some(market) = &common.market {
        query
            .append_pair("lang", &market.lang_code)
            .append_pair("countries", &market.country_code);

        if let Some(limit) = market.news_quality_rank_limit() {
            query.append_pair("to_rank", &limit.to_string());
        }
    }

    query
        .append_pair("page_size", &common.page_size.to_string())
        // FIXME Consider cmp::min(self.page, 1) or explicit error variant
        .append_pair("page", &common.page.to_string());

    if !common.trusted_sources.is_empty() {
        query.append_pair("sources", &common.trusted_sources.join(","));
    }

    if !common.excluded_sources.is_empty() {
        query.append_pair("not_sources", &common.excluded_sources.join(","));
    }
}

#[cfg(test)]
mod tests {
    use crate::{newscatcher::models::Topic, Filter, Market};

    use super::*;
    use chrono::NaiveDateTime;

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
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
        );

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/climate-change.json"
        ));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "\"Climate change\""))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "AU"))
            .and(query_param("page_size", "2"))
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

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(market),
                page_size: 2,
                page: 1,
                excluded_sources: &[],
                trusted_sources: &[],
            },
            filter,
            from: None,
        };

        let docs = provider.query_news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_simple_news_query_with_additional_parameters() {
        let mock_server = MockServer::start().await;
        let provider = NewsProviderImpl::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
        );

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/climate-change.json"
        ));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "\"Climate change\""))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("not_sources", "dodo.com,dada.net"))
            .and(query_param("to_rank", "9000"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market {
            lang_code: "de".to_string(),
            country_code: "DE".to_string(),
        };
        let filter = &Filter::default().add_keyword("Climate change");

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(market),
                page_size: 2,
                page: 1,
                excluded_sources: &["dodo.com".into(), "dada.net".into()],
                trusted_sources: &[],
            },
            filter,
            from: None,
        };

        let docs = provider.query_news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_multiple_keywords() {
        let mock_server = MockServer::start().await;
        let provider = NewsProviderImpl::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
        );

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/msft-vs-aapl.json"
        ));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "\"Bill Gates\" OR \"Tim Cook\""))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market {
            lang_code: "de".to_string(),
            country_code: "DE".to_string(),
        };
        let filter = &Filter::default()
            .add_keyword("Bill Gates")
            .add_keyword("Tim Cook");

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(market),
                page_size: 2,
                page: 1,
                excluded_sources: &[],
                trusted_sources: &[],
            },
            filter,
            from: None,
        };

        let docs = provider.query_news(&params).await.unwrap();
        assert_eq!(docs.len(), 2);

        let doc = docs.get(0).unwrap();
        assert_eq!(
            doc.title,
            "Porsche entwickelt Antrieb, der E-Mobilit\u{e4}t teilweise \u{fc}berlegen ist"
        );
    }

    #[tokio::test]
    async fn test_headlines() {
        let mock_server = MockServer::start().await;
        let provider = HeadlinesProviderImpl::new(
            Url::parse(&format!("{}/v1/latest-headlines", mock_server.uri())).unwrap(),
            "test-token".into(),
        );

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/latest-headlines.json"
        ));

        Mock::given(method("GET"))
            .and(path("/v1/latest-headlines"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "US"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("sources", "dodo.com,dada.net"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = Market {
            lang_code: "en".to_string(),
            country_code: "US".to_string(),
        };
        let params = HeadlinesQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 2,
                page: 1,
                excluded_sources: &[],
                trusted_sources: &["dodo.com".into(), "dada.net".into()],
            },
            topic: None,
            when: None,
        };

        let docs = provider.query_headlines(&params).await.unwrap();
        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        let expected = Article {
            title: "Jerusalem blanketed in white after rare snowfall".to_string(),
            score: None,
            rank: 6510,
            source_domain: "example.com".to_string(),
            snippet: "We use cookies. By Clicking \"OK\" or any content on this site, you agree to allow cookies to be placed. Read more in our privacy policy.".to_string(),
            url: "https://example.com".to_string(),
            image: "https://uploads.example.com/image.png".to_string(),
            topic: Topic::Gaming.to_string(),
            country: "US".to_string(),
            language: "en".to_string(),
            date_published: NaiveDateTime::parse_from_str("2022-01-27 13:24:33", "%Y-%m-%d %H:%M:%S").unwrap(),
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }
}
