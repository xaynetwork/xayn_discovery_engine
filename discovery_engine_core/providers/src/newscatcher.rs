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

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use derive_more::Deref;
use serde::{de, Deserialize, Deserializer, Serialize};
use url::Url;

use crate::{
    helpers::rest_endpoint::RestEndpoint,
    models::NewsQuery,
    Error,
    GenericArticle,
    HeadlinesProvider,
    HeadlinesQuery,
    Market,
    NewsProvider,
    RankLimit,
    TrustedHeadlinesProvider,
    TrustedHeadlinesQuery,
};

#[derive(Deref)]
pub struct NewscatcherNewsProvider {
    endpoint: RestEndpoint,
}

impl NewscatcherNewsProvider {
    pub fn new(endpoint_url: Url, auth_token: String, timeout: Duration, retry: usize) -> Self {
        Self {
            endpoint: RestEndpoint::new(endpoint_url, auth_token, timeout, retry),
        }
    }

    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn NewsProvider> {
        Arc::new(Self { endpoint })
    }
}

pub(crate) fn max_age_to_date_string(max_age_days: usize) -> String {
    // (lj): This _could_ overflow if we specified trillions of days, but I don't
    // think that's worth guarding against.
    let days = max_age_days as i64;

    let from = Utc::today() - chrono::Duration::days(days);
    from.format("%Y/%m/%d").to_string()
}

#[allow(clippy::unnecessary_wraps)]
pub(crate) fn to_generic_articles(articles: Vec<Article>) -> Result<Vec<GenericArticle>, Error> {
    let articles = articles
        .into_iter()
        .flat_map(GenericArticle::try_from)
        .collect();
    Ok(articles)
}

#[async_trait]
impl NewsProvider for NewscatcherNewsProvider {
    async fn query_news(&self, request: &NewsQuery<'_>) -> Result<Vec<GenericArticle>, Error> {
        let response = self
            .endpoint
            .get_request::<_, Response>(|query_append| {
                query_append("page_size", request.page_size.to_string());
                query_append("page", request.page.to_string());

                if !request.excluded_sources.is_empty() {
                    query_append("not_sources", request.excluded_sources.join(","));
                }

                query_append("sort_by", "relevancy".to_owned());
                append_market(query_append, request.market, &request.rank_limit);
                query_append("q", request.filter.build());

                if let Some(days) = &request.max_age_days {
                    query_append("from", max_age_to_date_string(*days));
                }
            })
            .await?;

        to_generic_articles(response.articles)
    }
}

pub struct NewscatcherHeadlinesProvider {
    endpoint: RestEndpoint,
}

impl NewscatcherHeadlinesProvider {
    /// Create a new provider.
    pub fn new(endpoint_url: Url, auth_token: String, timeout: Duration, retry: usize) -> Self {
        Self {
            endpoint: RestEndpoint::new(endpoint_url, auth_token, timeout, retry),
        }
    }

    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn HeadlinesProvider> {
        Arc::new(Self { endpoint })
    }
}

#[async_trait]
impl HeadlinesProvider for NewscatcherHeadlinesProvider {
    async fn query_headlines(
        &self,
        request: &HeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error> {
        let response = self
            .endpoint
            .get_request::<_, Response>(|query_append| {
                query_append("page_size", request.page_size.to_string());
                query_append("page", request.page.to_string());

                if !request.excluded_sources.is_empty() {
                    query_append("not_sources", request.excluded_sources.join(","));
                }

                append_market(query_append, request.market, &request.rank_limit);

                if let Some(days) = request.max_age_days {
                    query_append("when", format!("{}d", days));
                }

                if let Some(topic) = request.topic {
                    query_append("topic", topic.to_owned());
                }
            })
            .await?;

        to_generic_articles(response.articles)
    }
}

pub struct NewscatcherTrustedHeadlinesProvider {
    endpoint: RestEndpoint,
}

impl NewscatcherTrustedHeadlinesProvider {
    pub fn new(endpoint_url: Url, auth_token: String, timeout: Duration, retry: usize) -> Self {
        Self {
            endpoint: RestEndpoint::new(endpoint_url, auth_token, timeout, retry),
        }
    }

    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn TrustedHeadlinesProvider> {
        Arc::new(Self { endpoint })
    }
}

#[async_trait]
impl TrustedHeadlinesProvider for NewscatcherTrustedHeadlinesProvider {
    async fn query_trusted_sources(
        &self,
        request: &TrustedHeadlinesQuery<'_>,
    ) -> Result<Vec<GenericArticle>, Error> {
        let response = self
            .endpoint
            .get_request::<_, Response>(|query_append| {
                query_append("page_size", request.page_size.to_string());
                query_append("page", request.page.to_string());

                if !request.excluded_sources.is_empty() {
                    query_append("not_sources", request.excluded_sources.join(","));
                }

                if let Some(days) = request.max_age_days {
                    query_append("when", format!("{}d", days));
                }

                if !request.trusted_sources.is_empty() {
                    query_append("sources", request.trusted_sources.join(","));
                }
            })
            .await?;

        to_generic_articles(response.articles)
    }
}

pub(crate) fn append_market(
    query_append: &mut dyn FnMut(&str, String),
    market: &Market,
    rank_limit: &RankLimit,
) {
    query_append("lang", market.lang_code.clone());
    query_append("countries", market.country_code.clone());

    let rank_limit = (rank_limit, market.quality_rank_limit());
    if let (RankLimit::LimitedByMarket, Some(limit)) = rank_limit {
        query_append("to_rank", limit.to_string());
    }
}

/// A news article
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Article {
    /// The title of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub title: String,

    /// How well the article is matching your search criteria.
    #[serde(
        default,
        rename(deserialize = "_score"),
        deserialize_with = "deserialize_null_default"
    )]
    pub score: Option<f32>,

    /// The page rank of the source website.
    #[serde(default, deserialize_with = "deserialize_rank")]
    pub rank: u64,

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    #[serde(
        default,
        rename(deserialize = "clean_url"),
        deserialize_with = "deserialize_null_default"
    )]
    pub source_domain: String,

    /// Short summary of the article provided by the publisher.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub excerpt: String,

    /// Full URL where the article was originally published.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub link: String,

    /// A link to a thumbnail image of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub media: String,

    /// The main topic of the news publisher.
    /// Important: This parameter is not deducted on a per-article level:
    /// it is deducted on the per-publisher level.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub topic: String,

    /// The country of the publisher.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub country: String,

    /// The language of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub language: String,

    /// While Newscatcher claims to have some sort of timezone support in their
    /// [API][<https://docs.newscatcherapi.com/api-docs/endpoints/search-news>] (via the
    /// `published_date_precision` attribute), in practice they do not seem to be supplying any
    /// sort of timezone information. As a result, we provide NaiveDateTime for now.
    #[serde(
        default = "default_published_date",
        deserialize_with = "deserialize_naive_date_time_from_str"
    )]
    pub published_date: NaiveDateTime,

    /// Optional article embedding from the provider.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub embedding: Option<Vec<f32>>,
}

const fn default_published_date() -> NaiveDateTime {
    NaiveDateTime::MIN
}

// Taken from https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Null-value tolerant deserialization of `NaiveDateTime`
fn deserialize_naive_date_time_from_str<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    opt.map_or_else(
        || Ok(NaiveDateTime::from_timestamp(0, 0)),
        |s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(de::Error::custom),
    )
}

/// Null-value tolerant deserialization of rank
fn deserialize_rank<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(u64::MAX))
}

/// Query response from the Newscatcher API
#[derive(Deserialize, Debug)]
pub struct Response {
    /// Status message
    pub status: String,
    /// Main response content
    #[serde(default)]
    pub articles: Vec<Article>,
    /// Total pages of content available
    pub total_pages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    use crate::{Filter, HeadlinesQuery, Market, Rank, UrlWithDomain};

    use chrono::NaiveDateTime;

    use crate::models::RankLimit;
    use wiremock::{
        matchers::{header, method, path, query_param, query_param_is_missing},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    #[tokio::test]
    async fn test_simple_news_query() {
        let mock_server = MockServer::start().await;
        let provider = NewscatcherNewsProvider::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
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

        let market = &Market::new("en", "AU");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = NewsQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
        };

        let docs = provider.query_news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_rank_limit() {
        let mock_server = MockServer::start().await;
        let provider = NewscatcherNewsProvider::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "(Climate change)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "AT"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("to_rank", "70000"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market::new("de", "AT");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = NewsQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
        };

        let docs = provider.query_news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_simple_news_query_with_additional_parameters() {
        let mock_server = MockServer::start().await;
        let provider = NewscatcherNewsProvider::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/climate-change.json"));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "(Climate change)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("from", max_age_to_date_string(3)))
            .and(query_param("not_sources", "dodo.com,dada.net"))
            .and(query_param("to_rank", "9000"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market::new("de", "DE");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = NewsQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &["dodo.com".into(), "dada.net".into()],
            market,
            filter,
            max_age_days: Some(3),
        };

        let docs = provider.query_news(&params).await.unwrap();

        assert_eq!(docs.len(), 2);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_multiple_keywords() {
        let mock_server = MockServer::start().await;
        let provider = NewscatcherNewsProvider::new(
            Url::parse(&format!("{}/v1/search-news", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/msft-vs-aapl.json"));

        Mock::given(method("GET"))
            .and(path("/v1/search-news"))
            .and(query_param("q", "(Bill Gates) OR (Tim Cook)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let market = &Market::new("de", "DE");
        let filter = &Filter::default()
            .add_keyword("Bill Gates")
            .add_keyword("Tim Cook");

        let params = NewsQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
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
        let provider = NewscatcherHeadlinesProvider::new(
            Url::parse(&format!("{}/v1/latest-headlines", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path("/v1/latest-headlines"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "US"))
            .and(query_param("topic", "games"))
            .and(query_param("when", "3d"))
            // `sort_by` only supported by the news endpoint
            .and(query_param_is_missing("sort_by"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let params = HeadlinesQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market: &Market::new("en", "US"),
            topic: Some("games"),
            max_age_days: Some(3),
            trusted_sources: &[],
        };

        let docs = provider.query_headlines(&params).await.unwrap();
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
            date_published: NaiveDateTime::parse_from_str("2022-01-27 13:24:33", "%Y-%m-%d %H:%M:%S").unwrap(),
            country: "US".to_string(),
            language: "en".to_string(),
            embedding: None
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }

    #[tokio::test]
    async fn test_trusted_sources() {
        let mock_server = MockServer::start().await;
        let provider = NewscatcherTrustedHeadlinesProvider::new(
            Url::parse(&format!("{}/v2/trusted-sources", mock_server.uri())).unwrap(),
            "test-token".into(),
            Duration::from_secs(1),
            0,
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path("/v2/trusted-sources"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("sources", "dodo.com,dada.net"))
            .and(query_param("when", "3d"))
            // `sort_by` only supported by the news endpoint
            .and(query_param_is_missing("sort_by"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(tmpl)
            .expect(1)
            .mount(&mock_server)
            .await;

        let params = TrustedHeadlinesQuery {
            market: None,
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            trusted_sources: &["dodo.com".into(), "dada.net".into()],
            max_age_days: Some(3),
        };

        let docs = provider.query_trusted_sources(&params).await.unwrap();
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
            date_published: NaiveDateTime::parse_from_str("2022-01-27 13:24:33", "%Y-%m-%d %H:%M:%S").unwrap(),
            country: "US".to_string(),
            language: "en".to_string(),
            embedding: None
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }

    impl Default for Article {
        fn default() -> Self {
            Article {
                title: "title".to_string(),
                score: Some(0.75),
                rank: 10,
                source_domain: "example.com".to_string(),
                excerpt: "summary of the article".to_string(),
                link: "https://example.com/news/".to_string(),
                media: "https://example.com/news/image/".to_string(),
                topic: "news".to_string(),
                country: "US".to_string(),
                language: "en".to_string(),
                published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
                embedding: None,
            }
        }
    }

    #[test]
    // In order to make sure that our API clients don't throw errors if some articles
    // are malformed (missing fields, null fields) we are very liberal in what we
    // accept as articles, and will filter out malformed ones further down the processing
    // chain.
    fn test_deserialize_article_where_all_fields_should_fall_back_to_default() {
        let _article: Article = serde_json::from_str("{}").unwrap();
    }
}
