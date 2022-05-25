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

use chrono::Utc;
use itertools::Itertools;
use url::Url;

use crate::{
    filter::{Filter, Market},
    seal::Seal,
    Article,
    Error,
};

use self::models::Response;

mod models;

const TRUSTED_SOURCES_ENDPOINT: &str = "trusted-sources";

/// Represents a Query to Newscatcher.
pub trait NewscatcherQuery: Seal + Sync {
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
    /// Starting point in time from which to start the search.
    /// The format is YYYY/mm/dd. Default timezone is UTC.
    /// Defaults to the last week.
    pub from: Option<String>,
}

impl<F> NewscatcherQuery for NewsQuery<'_, F>
where
    F: Deref<Target = Filter> + Sync,
{
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        self.common.setup_url(url, "search-news")?;

        let mut query = url.query_pairs_mut();
        query
            .append_pair("sort_by", "relevancy")
            .append_pair("q", &self.filter.build());

        if let Some(from) = &self.from {
            query.append_pair("from", from);
        }

        Ok(())
    }
}

impl<T> Seal for NewsQuery<'_, T> {}

/// Parameters determining which headlines to fetch.
pub struct HeadlinesQuery<'a> {
    /// Common parts.
    pub common: CommonQueryParts<'a>,
    /// Trusted sources.
    pub trusted_sources: &'a [String],
    /// Headlines topic.
    pub topic: Option<&'a str>,
    /// The time period you want to get the latest headlines for.
    /// Can be specified in days (e.g. 3d) or hours (e.g. 24h).
    /// Defaults to all data available for the subscriptions.
    pub when: Option<&'a str>,
}

impl NewscatcherQuery for HeadlinesQuery<'_> {
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        self.common.setup_url(url, TRUSTED_SOURCES_ENDPOINT)?;

        let mut query = url.query_pairs_mut();
        if !self.trusted_sources.is_empty() {
            query.append_pair("sources", &self.trusted_sources.join(","));
        };
        if let Some(topic) = self.topic {
            query.append_pair("topic", topic);
        }
        if let Some(when) = self.when {
            query.append_pair("when", when);
        }
        Ok(())
    }
}

impl Seal for HeadlinesQuery<'_> {}

/// Client that can provide documents.
pub struct Client {
    token: String,
    url: String,
    timeout: Duration,
    client: reqwest::Client,
}

impl Client {
    /// Create a client.
    pub fn new(token: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            url: url.into(),
            timeout: Duration::from_millis(3500),
            client: reqwest::Client::new(),
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
    pub async fn query_articles(
        &self,
        query: &impl NewscatcherQuery,
    ) -> Result<Vec<Article>, Error> {
        let response = self.query(query).await?;
        Ok(response.articles.into_iter().map_into().collect())
    }

    /// Run a query against Newscatcher.
    async fn query(&self, query: &impl NewscatcherQuery) -> Result<Response, Error> {
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

/// Default `from` value for newscatcher news queries
pub fn default_from() -> String {
    let from = Utc::today() - chrono::Duration::days(3);
    from.format("%Y/%m/%d").to_string()
}

/// Default `when` value for newscatcher headline queries
pub const DEFAULT_WHEN: Option<&'static str> = Some("3d");

#[cfg(test)]
mod tests {
    use crate::newscatcher::models::Topic;

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
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/climate-change.json"
        ));

        Mock::given(method("GET"))
            .and(path("/search-news"))
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
            from: None,
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

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/climate-change.json"
        ));

        Mock::given(method("GET"))
            .and(path("/search-news"))
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
            },
            filter,
            from: None,
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

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/msft-vs-aapl.json"
        ));

        Mock::given(method("GET"))
            .and(path("/search-news"))
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
            from: None,
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

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/newscatcher/latest-headlines.json"
        ));

        Mock::given(method("GET"))
            .and(path(TRUSTED_SOURCES_ENDPOINT))
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
            trusted_sources: &["dodo.com".into(), "dada.net".into()],
            topic: None,
            when: None,
        };

        let docs = client.query_articles(&params).await.unwrap();
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
