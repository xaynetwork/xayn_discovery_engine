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

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use derive_more::Deref;
use serde::{de, Deserialize, Deserializer};

use crate::{
    error::Error,
    models::{
        content::GenericArticle,
        query::{HeadlinesQuery, RankLimit, SearchQuery, TrustedHeadlinesQuery},
    },
    utils::{filter::Market, rest_endpoint::RestEndpoint},
    HeadlinesProvider,
    SearchProvider,
    TrustedHeadlinesProvider,
};

#[derive(Deref)]
pub struct NewscatcherSearchProvider {
    endpoint: RestEndpoint,
}

impl NewscatcherSearchProvider {
    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn SearchProvider> {
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
impl SearchProvider for NewscatcherSearchProvider {
    async fn query_search(&self, request: &SearchQuery<'_>) -> Result<Vec<GenericArticle>, Error> {
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
    pub fn from_endpoint(endpoint: RestEndpoint) -> Arc<dyn TrustedHeadlinesProvider> {
        Arc::new(Self { endpoint })
    }
}

#[async_trait]
impl TrustedHeadlinesProvider for NewscatcherTrustedHeadlinesProvider {
    async fn query_trusted_headlines(
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
#[derive(Clone, Debug, Deserialize)]
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
    /// sort of timezone information. As a result, we provide DateTime<Utc> for now.
    #[serde(
        default = "default_published_date",
        rename(deserialize = "published_date"),
        deserialize_with = "deserialize_date_time_utc_from_str"
    )]
    pub date_published: DateTime<Utc>,

    /// Optional article embedding from the provider.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub embedding: Option<Vec<f32>>,
}

const fn default_published_date() -> DateTime<Utc> {
    DateTime::<Utc>::MIN_UTC
}

/// Null-value tolerant deserialization of `Option<T: Default>`.
// see <https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617>
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// Null-value tolerant deserialization of `DateTime<Utc>`.
fn deserialize_date_time_utc_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?.map_or_else(
        || Ok(Utc.ymd(1970, 1, 1).and_hms(0, 0, 0)),
        |s| {
            Utc.datetime_from_str(&s, "%Y-%m-%d %H:%M:%S")
                .map_err(de::Error::custom)
        },
    )
}

/// Null-value tolerant deserialization of rank.
fn deserialize_rank<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    Option::deserialize(deserializer).map(|option| option.unwrap_or(u64::MAX))
}

/// Query response from the Newscatcher API.
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
    use url::Url;
    use wiremock::{
        matchers::{header, method, path, query_param, query_param_is_missing},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    use crate::{
        config::Config,
        models::{
            content::{Rank, UrlWithDomain},
            query::{HeadlinesQuery, RankLimit},
        },
        Filter,
        Market,
    };

    use super::*;

    #[tokio::test]
    async fn test_simple_news_query() {
        let server = MockServer::start().await;
        let route = Config::SEARCH;
        let token = "test-token";
        let provider = NewscatcherSearchProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/search-news.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("q", "(Climate change)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "AU"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
            .await;

        let market = &Market::new("en", "AU");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = SearchQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
        };

        let docs = provider.query_search(&params).await.unwrap();

        assert_eq!(docs.len(), 4);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_rank_limit() {
        let server = MockServer::start().await;
        let route = Config::SEARCH;
        let token = "test-token";
        let provider = NewscatcherSearchProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/search-news.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("q", "(Climate change)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "AT"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("to_rank", "70000"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
            .await;

        let market = &Market::new("de", "AT");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = SearchQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
        };

        let docs = provider.query_search(&params).await.unwrap();

        assert_eq!(docs.len(), 4);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_simple_news_query_with_additional_parameters() {
        let server = MockServer::start().await;
        let route = Config::SEARCH;
        let token = "test-token";
        let provider = NewscatcherSearchProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/search-news.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("q", "(Climate change)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("from", max_age_to_date_string(3)))
            .and(query_param("not_sources", "dodo.com,dada.net"))
            .and(query_param("to_rank", "9000"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
            .await;

        let market = &Market::new("de", "DE");
        let filter = &Filter::default().add_keyword("Climate change");

        let params = SearchQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &["dodo.com".into(), "dada.net".into()],
            market,
            filter,
            max_age_days: Some(3),
        };

        let docs = provider.query_search(&params).await.unwrap();

        assert_eq!(docs.len(), 4);

        let doc = docs.get(1).unwrap();
        assert_eq!(doc.title, "Businesses \u{2018}more concerned than ever'");
    }

    #[tokio::test]
    async fn test_news_multiple_keywords() {
        let server = MockServer::start().await;
        let route = Config::SEARCH;
        let token = "test-token";
        let provider = NewscatcherSearchProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/msft-vs-aapl.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("q", "(Bill Gates) OR (Tim Cook)"))
            .and(query_param("sort_by", "relevancy"))
            .and(query_param("lang", "de"))
            .and(query_param("countries", "DE"))
            .and(query_param("page_size", "2"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
            .await;

        let market = &Market::new("de", "DE");
        let filter = &Filter::default()
            .add_keyword("Bill Gates")
            .add_keyword("Tim Cook");

        let params = SearchQuery {
            page_size: 2,
            page: 1,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            market,
            filter,
            max_age_days: None,
        };

        let docs = provider.query_search(&params).await.unwrap();
        assert_eq!(docs.len(), 2);

        let doc = docs.get(0).unwrap();
        assert_eq!(
            doc.title,
            "Porsche entwickelt Antrieb, der E-Mobilit\u{e4}t teilweise \u{fc}berlegen ist"
        );
    }

    #[tokio::test]
    async fn test_headlines() {
        let server = MockServer::start().await;
        let route = Config::HEADLINES;
        let token = "test-token";
        let provider = NewscatcherHeadlinesProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("lang", "en"))
            .and(query_param("countries", "US"))
            .and(query_param("topic", "games"))
            .and(query_param("when", "3d"))
            // `sort_by` only supported by the news endpoint
            .and(query_param_is_missing("sort_by"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
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
        assert_eq!(docs.len(), 4);

        let doc = docs.get(1).unwrap();
        let expected = GenericArticle {
            title: "Jerusalem blanketed in white after rare snowfall".to_string(),
            score: None,
            rank: Rank::new(6510),
            snippet: "We use cookies. By Clicking \"OK\" or any content on this site, you agree to allow cookies to be placed. Read more in our privacy policy.".to_string(),
            url: UrlWithDomain::parse("https://example.com/a/2").unwrap(),
            image: Some(Url::parse("https://uploads.example.com/image2.png").unwrap()),
            topic: "gaming".to_string(),
            date_published: Utc.ymd(2022, 1, 27).and_hms(13, 24, 33),
            country: "US".to_string(),
            language: "en".to_string(),
            embedding: None
        };

        assert_eq!(format!("{:?}", doc), format!("{:?}", expected));
    }

    #[tokio::test]
    async fn test_trusted_sources() {
        let server = MockServer::start().await;
        let route = Config::TRUSTED_HEADLINES;
        let token = "test-token";
        let provider = NewscatcherTrustedHeadlinesProvider::from_endpoint(
            Config::new(&server.uri(), route, token, false)
                .unwrap()
                .build(),
        );

        let tmpl = ResponseTemplate::new(200)
            .set_body_string(include_str!("../test-fixtures/latest-headlines.json"));

        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("page_size", "2"))
            .and(query_param("page", "1"))
            .and(query_param("sources", "dodo.com,dada.net"))
            .and(query_param("when", "3d"))
            // `sort_by` only supported by the news endpoint
            .and(query_param_is_missing("sort_by"))
            .and(header("Authorization", format!("Bearer {token}").as_str()))
            .respond_with(tmpl)
            .expect(1)
            .mount(&server)
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

        let docs = provider.query_trusted_headlines(&params).await.unwrap();
        assert_eq!(docs.len(), 4);

        let doc = docs.get(1).unwrap();
        let expected = GenericArticle {
            title: "Jerusalem blanketed in white after rare snowfall".to_string(),
            score: None,
            rank: Rank::new(6510),
            snippet: "We use cookies. By Clicking \"OK\" or any content on this site, you agree to allow cookies to be placed. Read more in our privacy policy.".to_string(),
            url: UrlWithDomain::parse("https://example.com/a/2").unwrap(),
            image: Some(Url::parse("https://uploads.example.com/image2.png").unwrap()),
            topic: "gaming".to_string(),
            date_published: Utc.ymd(2022, 1, 27).and_hms(13, 24, 33),
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
                date_published: Utc.ymd(2022, 1, 1).and_hms(9, 0, 0),
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
