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

use std::{ops::Deref, time::Duration};

use displaydoc::Display as DisplayDoc;
use thiserror::Error;
use url::Url;

use crate::{
    filter::{Filter, Market},
    newscatcher::{Article, Response as NewscatcherResponse},
    seal::Seal,
};

/// Client errors.
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Invalid API Url base
    InvalidUrlBase(Option<url::ParseError>),
    /// Failed to execute the HTTP request: {0}
    RequestExecution(#[source] reqwest::Error),
    /// Server returned a non-successful status code: {0}
    StatusCode(#[source] reqwest::Error),
    /// Failed to fetch from the server: {0}
    Fetching(#[source] reqwest::Error),
    /// Failed to decode the server's response: {0}
    Decoding(#[source] serde_json::Error),
    /// Failed to decode the server's response at JSON path {1}: {0}
    DecodingAtPath(
        String,
        #[source] serde_path_to_error::Error<serde_json::Error>,
    ),
}

/// Represents a Query to Newscatcher.
pub trait Query: Seal + Sync {
    /// Sets query specific parameters on given Newscatcher base URL.
    fn setup_url(&self, url: &mut Url) -> Result<(), Error>;
}

/// Elements shared between various Newscatcher queries.
pub struct CommonQueryParts<'a> {
    /// Market of news.
    pub market: Option<&'a Market>,
    /// How many articles to return (per page).
    pub page_size: usize,
    /// The number of the page which should be returned.
    ///
    /// Paging starts with `1`.
    pub page: usize,
    /// Exclude given sources.
    pub excluded_sources: &'a [String],
}

impl CommonQueryParts<'_> {
    fn setup_url(&self, url: &mut Url, single_path_element_suffix: &str) -> Result<(), Error> {
        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrlBase(None))?
            .push(single_path_element_suffix);

        let query = &mut url.query_pairs_mut();

        if let Some(market) = &self.market {
            query
                .append_pair("lang", &market.lang_code)
                .append_pair("countries", &market.country_code);

            if let Some(limit) = market.news_quality_rank_limit() {
                query.append_pair("to_rank", &limit.to_string());
            }
        }

        query
            .append_pair("page_size", &self.page_size.to_string())
            // FIXME Consider cmp::min(self.page, 1) or explicit error variant
            .append_pair("page", &self.page.to_string());

        if !self.excluded_sources.is_empty() {
            query.append_pair("not_sources", &self.excluded_sources.join(","));
        }

        Ok(())
    }
}

/// Parameters determining which news to fetch
pub struct NewsQuery<'a, F> {
    /// Common parts
    pub common: CommonQueryParts<'a>,
    /// News filter.
    pub filter: F,
}

impl<F> Query for NewsQuery<'_, F>
where
    F: Deref<Target = Filter> + Sync,
{
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        self.common.setup_url(url, "_sn")?;

        let mut query = url.query_pairs_mut();
        query
            .append_pair("sort_by", "relevancy")
            .append_pair("q", &self.filter.build());

        Ok(())
    }
}

impl<T> Seal for NewsQuery<'_, T> {}

/// Parameters determining which headlines to fetch.
pub struct HeadlinesQuery<'a> {
    /// Common parts.
    pub common: CommonQueryParts<'a>,
    /// Favourite sources.
    pub sources: &'a [String],
}

impl Query for HeadlinesQuery<'_> {
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        self.common.setup_url(url, "_lh")?;

        if !self.sources.is_empty() {
            let mut query = url.query_pairs_mut();
            query.append_pair("sources", &self.sources.join(","));
        };

        Ok(())
    }
}

impl Seal for HeadlinesQuery<'_> {}

/// Client that can provide documents.
pub struct Client {
    token: String,
    url: String,
    timeout: Duration,
}

impl Client {
    /// Create a client.
    pub fn new(token: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            url: url.into(),
            timeout: Duration::from_millis(3500),
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

    /// Run a query for fetching `Article`s from Newscatcher.
    pub async fn query_articles(&self, query: &impl Query) -> Result<Vec<Article>, Error> {
        self.query_newscatcher(query)
            .await
            .map(|news| news.articles)
    }

    /// Run a query against Newscatcher.
    pub async fn query_newscatcher(
        &self,
        query: &impl Query,
    ) -> Result<NewscatcherResponse, Error> {
        let mut url = Url::parse(&self.url).map_err(|e| Error::InvalidUrlBase(Some(e)))?;
        query.setup_url(&mut url)?;

        let c = reqwest::Client::new();
        let response = c
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    use crate::newscatcher::Topic;
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    #[tokio::test]
    async fn test_simple_news_query() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/_sn"))
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
            },
            filter,
        };

        let docs = client.query_articles(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_simple_news_query_with_additional_parameters() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/_sn"))
            .and(query_param("q", "\"Climate change\""))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("not_sources", "dodo.com,dada.net"))
            .and(query_param("to_rank", "12000"))
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
            },
            filter,
        };

        let docs = client.query_articles(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_multiple_keywords() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/msft-vs-aapl.json"));

        Mock::given(method("GET"))
            .and(path("/_sn"))
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
            },
            filter,
        };

        let docs = client.query_articles(&params).await.unwrap();
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
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path("/_lh"))
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
            },
            sources: &["dodo.com".into(), "dada.net".into()],
        };

        let docs = client.query_articles(&params).await.unwrap();
        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        let expected = Article {
            id: "0251ae9f73ec12f4d3eced9c4dc9ccc8".to_string(),
            title: "Jerusalem blanketed in white after rare snowfall".to_string(),
            score: None,
            rank: 6510,
            source_domain: "xayn.com".to_string(),
            excerpt: "We use cookies. By Clicking \"OK\" or any content on this site, you agree to allow cookies to be placed. Read more in our privacy policy.".to_string(),
            link: "https://xayn.com".to_string(),
            media: "https://uploads-ssl.webflow.com/5ea197660b956f76d26f0026/6179684043a88260009773cd_hero-phone.png".to_string(),
            topic: Topic::Gaming,
            country: "US".to_string(),
            language: "en".to_string(),
            published_date: NaiveDateTime::parse_from_str("2022-01-27 13:24:33", "%Y-%m-%d %H:%M:%S").unwrap(),
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }
}
