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
use tracing::trace;
use url::Url;

use crate::{
    filter::{Filter, Market},
    newscatcher::Response as NewscatcherResponse,
    seal::Seal,
    Error,
    GenericArticle,
};

/// Represents a Query to Newscatcher.
pub trait Query: Seal + Sync {
    /// Sets query specific parameters on given Newscatcher base URL.
    fn setup_url(&self, url: &mut Url) -> Result<(), Error>;
}

/// Page rank limiting strategy.
pub enum RankLimit {
    LimitedByMarket,
    Unlimited,
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
    /// Page rank limiting strategy.
    pub rank_limit: RankLimit,
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

            let rank_limit = (&self.rank_limit, market.news_quality_rank_limit());
            if let (RankLimit::LimitedByMarket, Some(limit)) = rank_limit {
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
    /// Maximum age of news items we want to include in the results
    pub max_age_days: Option<usize>,
}

impl<F> NewsQuery<'_, F> {
    fn compute_from(&self) -> Option<String> {
        self.max_age_days.map(|days| {
            // (lj): This _could_ overflow if we specified trillions of days, but I don't
            // think that's worth guarding against.
            let days = days as i64;

            let from = Utc::today() - chrono::Duration::days(days);
            from.format("%Y/%m/%d").to_string()
        })
    }
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

        if let Some(from) = &self.compute_from() {
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
    /// Maximum age of news items we want to include in the results
    pub max_age_days: Option<usize>,
}

impl HeadlinesQuery<'_> {
    fn compute_when(&self) -> Option<String> {
        self.max_age_days.map(|days| format!("{}d", days))
    }
}

impl Query for HeadlinesQuery<'_> {
    fn setup_url(&self, url: &mut Url) -> Result<(), Error> {
        self.common.setup_url(url, "_lh")?;

        let mut query = url.query_pairs_mut();
        if !self.trusted_sources.is_empty() {
            query.append_pair("sources", &self.trusted_sources.join(","));
        };
        if let Some(topic) = self.topic {
            query.append_pair("topic", topic);
        }
        if let Some(when) = &self.compute_when() {
            query.append_pair("when", when);
        }
        Ok(())
    }
}

impl Seal for HeadlinesQuery<'_> {}

/// Client that can provide documents.
pub struct Client {
    pub(crate) token: String,
    pub(crate) url: String,
    pub(crate) timeout: Duration,
    pub(crate) client: reqwest::Client,
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
    pub async fn query_articles(&self, query: &impl Query) -> Result<Vec<GenericArticle>, Error> {
        let articles = self
            .query_newscatcher(query)
            .await
            .map(|news| news.articles)?;

        // If we don't receive any articles from the news source, we don't treat this
        // as an error...
        if articles.is_empty() {
            return Ok(vec![]);
        }

        let generic_articles: Vec<GenericArticle> = articles
            .into_iter()
            .filter_map(|article| {
                GenericArticle::try_from(article.clone())
                    .map_err(|e| {
                        trace!(
                            "Malformed article could not be convert ({:?}): {:?}",
                            e,
                            article
                        )
                    })
                    .ok()
            })
            .collect();

        // ... but if we _did_ receive articles from the news source, but couldn't convert
        // any of them into GenericArticles (which likely means they're malformed in some way)
        // then we _do_ treat that as an error.
        if generic_articles.is_empty() {
            return Err(Error::NoValidArticles);
        }

        Ok(generic_articles)
    }

    /// Run a query against Newscatcher.
    pub async fn query_newscatcher(
        &self,
        query: &impl Query,
    ) -> Result<NewscatcherResponse, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use claim::assert_matches;

    use crate::{Rank, UrlWithDomain};
    use wiremock::{
        matchers::{header, method, path, query_param, query_param_is_missing},
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
            .and(query_param("q", "(Climate change)"))
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
                rank_limit: RankLimit::LimitedByMarket,
                excluded_sources: &[],
            },
            filter,
            max_age_days: None,
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
            .and(query_param("q", "(Climate change)"))
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
                rank_limit: RankLimit::LimitedByMarket,
                excluded_sources: &["dodo.com".into(), "dada.net".into()],
            },
            filter,
            max_age_days: None,
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

        let from = (Utc::today() - chrono::Duration::days(30))
            .format("%Y/%m/%d")
            .to_string();
        Mock::given(method("GET"))
            .and(path("/_sn"))
            .and(query_param("q", "(Bill Gates) OR (Tim Cook)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(query_param("from", from))
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
                rank_limit: RankLimit::LimitedByMarket,
                excluded_sources: &[],
            },
            filter,
            max_age_days: Some(30),
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
            .and(query_param("when", "3d"))
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
        let trusted_sources = &["dodo.com".into(), "dada.net".into()];
        let params = HeadlinesQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 2,
                page: 1,
                rank_limit: RankLimit::LimitedByMarket,
                excluded_sources: &[],
            },
            trusted_sources,
            topic: None,
            max_age_days: Some(3),
        };

        let docs = client.query_articles(&params).await.unwrap();
        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        let expected = GenericArticle {
            title: "Jerusalem blanketed in white after rare snowfall".to_string(),
            score: None,
            rank: Rank::new(6510),
            snippet: "We use cookies. By Clicking \"OK\" or any content on this site, you agree to allow cookies to be placed. Read more in our privacy policy.".to_string(),
            url: UrlWithDomain::parse("https://example.com").unwrap(),
            image: Some(Url::parse("https://uploads.example.com/image.png").unwrap()),
            topic: "gaming".to_string(),
            country: "US".to_string(),
            language: "en".to_string(),
            date_published: NaiveDateTime::parse_from_str("2022-01-27 13:24:33", "%Y-%m-%d %H:%M:%S").unwrap(),
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }

    #[tokio::test]
    async fn test_common_query_rank_unlimited() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/_sn"))
            .and(query_param_is_missing("to_rank"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = ("DE", "de").into();
        let filter = &Filter::default().add_keyword("");

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 2,
                page: 1,
                rank_limit: RankLimit::Unlimited,
                excluded_sources: &[],
            },
            filter,
            max_age_days: None,
        };

        client.query_articles(&params).await.unwrap();
    }

    #[tokio::test]
    async fn test_valid_articles_should_be_preserved_invalid_ones_rejected() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200).set_body_string(include_str!(
            "../test-fixtures/invalid-and-valid-articles-mixed.json"
        ));

        Mock::given(method("GET"))
            .and(path("/_sn"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = ("DE", "de").into();
        let filter = &Filter::default().add_keyword("");

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 2,
                page: 1,
                rank_limit: RankLimit::Unlimited,
                excluded_sources: &[],
            },
            filter,
            max_age_days: None,
        };

        let articles = client.query_articles(&params).await.unwrap();
        // Out of the three articles, only one is valid, and that one we want to return
        assert_eq!(articles.len(), 1);
        assert_eq!("valid article", articles[0].title);
    }

    #[tokio::test]
    async fn test_no_valid_articles_yields_an_error() {
        let mock_server = MockServer::start().await;
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/invalid-articles-only.json"));

        Mock::given(method("GET"))
            .and(path("/_sn"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = ("DE", "de").into();
        let filter = &Filter::default().add_keyword("");

        let params = NewsQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 2,
                page: 1,
                rank_limit: RankLimit::Unlimited,
                excluded_sources: &[],
            },
            filter,
            max_age_days: None,
        };

        let result = client.query_articles(&params).await;
        assert_matches!(result, Err(Error::NoValidArticles));
    }
}
