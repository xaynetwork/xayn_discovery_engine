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

use std::{collections::BTreeMap, time::Duration};

use displaydoc::Display as DisplayDoc;
use thiserror::Error;

use crate::{
    filter::{Filter, Market},
    newscatcher::{Article, Response as NewscatcherResponse},
};

#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Failed to execute the HTTP request: {0}
    RequestExecution(#[source] reqwest::Error),
    /// Server returned a non-successful status code: {0}
    StatusCode(#[source] reqwest::Error),
    /// Failed to decode the server's response: {0}
    Decoding(#[source] reqwest::Error),
}

/// Client that can provide documents.
// TODO the only reason `token`, `url` are `Default` is because we stringy typed them
//      instead of `Client` being default the place where `Client` is used should use
//      `Option<Client>` or better "configure" Ops when they are created
#[derive(Default)]
pub struct Client {
    token: String,
    // TODO make it a Url type
    url: String,
}

/// Parameters determining which news to fetch
pub struct NewsQuery<'a> {
    /// Market of news.
    pub market: &'a Market,
    /// News filter.
    pub filter: &'a Filter,
    /// How many articles to return (per page).
    pub page_size: usize,
    /// Page number.
    pub page: Option<usize>,
}

/// Parameters determining which headlines to fetch
pub struct HeadlinesQuery<'a> {
    /// Market of headlines.
    pub market: &'a Market,
    /// How many articles to return (per page).
    pub page_size: usize,
}

impl Client {
    const TIMEOUT: Duration = Duration::from_secs(15);

    /// Create a client.
    pub fn new(token: String, url: String) -> Self {
        Self { token, url }
    }

    /// Retrieve news from the remote API
    pub async fn news(&self, params: &NewsQuery<'_>) -> Result<Vec<Article>, Error> {
        //TODO this code can be largely de-duplicate, also
        //     `news`/`headlines` is specific to the caller.
        //      I.e. something like `pub async fn fetch_articles(path, impl FnOnce(&mut Url)` could do
        //          (and then add query parameters using Url::query_pairs_mut which also safes us the
        //           `to_string()`, `into()` calls for many values)
        let mut query: BTreeMap<String, String> = BTreeMap::new();
        query.insert("sort_by".into(), "relevancy".into());
        Self::build_news_query(&mut query, params);

        let c = reqwest::Client::new();
        let response = c
            //TODO use proper url path extension
            .get(format!("{}/_sn", self.url))
            .timeout(Self::TIMEOUT)
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(Error::RequestExecution)?
            .error_for_status()
            .map_err(Error::StatusCode)?;

        let news: NewscatcherResponse = response.json().await.map_err(Error::Decoding)?;
        let result: Vec<Article> = news.articles.into_iter().collect();
        Ok(result)
    }

    fn build_news_query(query: &mut BTreeMap<String, String>, params: &NewsQuery<'_>) {
        query.insert("lang".to_string(), params.market.lang_code.to_string());
        query.insert(
            "countries".to_string(),
            params.market.country_code.to_string(),
        );
        query.insert("page_size".to_string(), params.page_size.to_string());
        query.insert("q".to_string(), params.filter.build());
        if let Some(page) = params.page {
            query.insert("page".to_string(), page.to_string());
        }
    }

    /// Retrieve headlines from the remote API
    pub async fn headlines(&self, params: &HeadlinesQuery<'_>) -> Result<Vec<Article>, Error> {
        let mut query: BTreeMap<String, String> = BTreeMap::new();
        Self::build_headlines_query(&mut query, params);

        let c = reqwest::Client::new();
        let response = c
            .get(format!("{}/_lh", self.url))
            .timeout(Self::TIMEOUT)
            .bearer_auth(&self.token)
            .query(&query)
            .send()
            .await
            .map_err(Error::RequestExecution)?
            .error_for_status()
            .map_err(Error::StatusCode)?;

        let news: NewscatcherResponse = response.json().await.map_err(Error::Decoding)?;
        let result: Vec<Article> = news.articles.into_iter().collect();
        Ok(result)
    }

    fn build_headlines_query(query: &mut BTreeMap<String, String>, params: &HeadlinesQuery<'_>) {
        query.insert("lang".to_string(), params.market.lang_code.to_string());
        query.insert(
            "countries".to_string(),
            params.market.country_code.to_string(),
        );
        query.insert("page_size".to_string(), params.page_size.to_string());
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
        let client = Client {
            token: "test-token".to_string(),
            url: mock_server.uri(),
        };

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
            market,
            filter,
            page_size: 2,
            page: Some(1),
        };

        let docs = client.news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_multiple_keywords() {
        let mock_server = MockServer::start().await;
        let client = Client {
            token: "test-token".to_string(),
            url: mock_server.uri(),
        };

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
            market,
            filter,
            page_size: 2,
            page: None,
        };

        let docs = client.news(&params).await.unwrap();
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
        let client = Client {
            token: "test-token".to_string(),
            url: mock_server.uri(),
        };

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path("/_lh"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "US"))
            .and(query_param("page_size", "2"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let params = HeadlinesQuery {
            market: &Market {
                lang_code: "en".to_string(),
                country_code: "US".to_string(),
            },
            page_size: 2,
        };

        let docs = client.headlines(&params).await.unwrap();
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
