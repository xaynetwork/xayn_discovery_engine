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

use std::time::Duration;

use url::Url;

use crate::{
    client::Error,
    filter::{Filter, Market},
    gnews::{Article, Response},
};

const LATEST_HEADLINE_ENDPOINT: &str = "latest-headlines";
const SEARCH_NEWS_ENDPOINT: &str = "search-news";

/// Query parameters for the news search query
pub struct NewsQuery<'a> {
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
    /// Search filter for selecting the results
    pub filter: &'a Filter,
}

/// Query parameters for the headlines query
pub struct HeadlinesQuery<'a> {
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
    /// Search filter for selecting the results
    pub filter: Option<&'a Filter>,
}

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

    /// Run a query for fetching `Article`s
    pub async fn query_articles(&self, query: &NewsQuery<'_>) -> Result<Vec<Article>, Error> {
        let url = self.build_news_url(query)?;
        self.query(url).await.map(|news| news.articles)
    }

    /// Run a query for fetching `Article`s
    pub async fn query_headlines(&self, query: &HeadlinesQuery<'_>) -> Result<Vec<Article>, Error> {
        let url = self.build_headlines_url(query)?;
        self.query(url).await.map(|news| news.articles)
    }

    /// Run a query against the gnews API.
    pub async fn query(&self, url: Url) -> Result<Response, Error> {
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

    fn build_news_url(&self, params: &NewsQuery<'_>) -> Result<Url, Error> {
        let mut url = Url::parse(&self.url).map_err(|e| Error::InvalidUrlBase(Some(e)))?;

        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrlBase(None))?
            .push(SEARCH_NEWS_ENDPOINT);

        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("sortby", "relevance")
                .append_pair("q", &params.filter.build());

            if let Some(market) = &params.market {
                query
                    .append_pair("lang", &market.lang_code)
                    .append_pair("country", &market.country_code);
            }

            query
                .append_pair("max", &params.page_size.to_string())
                .append_pair("page", &params.page.to_string());
        }

        Ok(url)
    }

    fn build_headlines_url(&self, params: &HeadlinesQuery<'_>) -> Result<Url, Error> {
        let mut url = Url::parse(&self.url).map_err(|e| Error::InvalidUrlBase(Some(e)))?;

        url.path_segments_mut()
            .map_err(|_| Error::InvalidUrlBase(None))?
            .push(LATEST_HEADLINE_ENDPOINT);

        {
            let mut query = url.query_pairs_mut();
            query.append_pair("sortby", "relevance");

            if let Some(filter) = &params.filter {
                query.append_pair("q", &filter.build());
            }

            if let Some(market) = &params.market {
                query
                    .append_pair("lang", &market.lang_code)
                    .append_pair("country", &market.country_code);
            }

            query
                .append_pair("max", &params.page_size.to_string())
                .append_pair("page", &params.page.to_string());
        }

        Ok(url)
    }
}

#[cfg(test)]
mod tests {
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
        let client = Client::new("test-token", mock_server.uri());

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/gnews/climate-change.json"));

        Mock::given(method("GET"))
            .and(path(SEARCH_NEWS_ENDPOINT))
            .and(query_param("q", "\"Climate change\""))
            .and(query_param("sortby", "relevance"))
            .and(query_param("lang", "en"))
            .and(query_param("country", "AU"))
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

        let params = NewsQuery {
            market: Some(market),
            page_size: 2,
            page: 1,
            excluded_sources: &[],
            filter,
        };

        let docs = client.query_articles(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(
            doc.title,
            "[WATCH] #StoryOfTheNation: Environment, climate change, and the 2022 polls"
        );
    }
}
