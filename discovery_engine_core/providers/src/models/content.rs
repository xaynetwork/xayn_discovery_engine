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

use chrono::{DateTime, Utc};
use derive_more::Deref;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{error::Error, newscatcher::Article as NewscatcherArticle};

/// A helper type used to ensure that, within the [`GenericArticle`] struct,
/// we never use a URL which does not have a domain, such as `file:///foo/bar`.
#[derive(Debug, Serialize, Deserialize, Clone, Deref)]
pub struct UrlWithDomain(Url);

impl UrlWithDomain {
    pub fn new(url: Url) -> Option<Self> {
        if url.domain().is_none() {
            None
        } else {
            Some(UrlWithDomain(url))
        }
    }

    pub fn parse(input: &str) -> Result<Self, Error> {
        let url = Url::parse(input).map_err(Error::InvalidUrl)?;
        UrlWithDomain::new(url).ok_or_else(|| Error::MissingDomainInUrl(input.to_string()))
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn domain(&self) -> &str {
        self.0.domain().unwrap(/* constructor makes sure we have a domain */)
    }

    pub fn to_inner(&self) -> Url {
        self.0.clone()
    }
}

impl TryFrom<&str> for UrlWithDomain {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        UrlWithDomain::parse(value)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Deref, Default)]
pub struct Rank(pub u64);

impl Rank {
    pub fn new(rank: u64) -> Self {
        Rank(rank)
    }
}

/// Represents an article that is delivered by an external content API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericArticle {
    pub title: String,
    pub snippet: String,
    pub url: UrlWithDomain,
    pub date_published: DateTime<Utc>,
    pub country: String,
    pub language: String,
    pub topic: String,
    pub image: Option<Url>,
    pub rank: Rank,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// Optional article embedding from the provider.
    pub embedding: Option<Vec<f32>>,
}

impl GenericArticle {
    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub fn source_domain(&self) -> String {
        self.url.domain().to_string()
    }

    /// Gets the snippet or falls back to the title if the snippet is empty.
    pub fn snippet_or_title(&self) -> &str {
        if self.snippet.is_empty() {
            &self.title
        } else {
            &self.snippet
        }
    }
}

impl TryFrom<NewscatcherArticle> for GenericArticle {
    type Error = Error;
    fn try_from(article: NewscatcherArticle) -> Result<Self, Self::Error> {
        let media = article.media;
        let image = (!media.is_empty())
            .then(|| Url::parse(&media))
            .transpose()?;

        let url = UrlWithDomain::parse(&article.link)?;
        Ok(Self {
            title: article.title,
            snippet: article.excerpt,
            date_published: article.date_published,
            url,
            image,
            rank: Rank::new(article.rank),
            score: article.score,
            country: article.country,
            language: article.language,
            topic: article.topic,
            embedding: article.embedding,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    impl Default for GenericArticle {
        fn default() -> Self {
            GenericArticle {
                title: String::default(),
                snippet: String::default(),
                url: example_url(),
                image: None,
                date_published: Utc.with_ymd_and_hms(2022, 1, 1, 9, 0, 0).unwrap(),
                score: None,
                rank: Rank::default(),
                country: "US".to_string(),
                language: "en".to_string(),
                topic: "news".to_string(),
                embedding: None,
            }
        }
    }

    fn example_url() -> UrlWithDomain {
        let url = Url::parse("https://example.net").unwrap(/* used only in tests */);
        UrlWithDomain::new(url).unwrap(/* used only in tests */)
    }

    #[test]
    fn test_url_with_domain() {
        let url = Url::parse("file:///foo/bar").unwrap();
        let wrapped = UrlWithDomain::new(url);
        assert!(wrapped.is_none());
    }

    #[test]
    fn test_source_domain_extraction() {
        let res = GenericArticle::default();
        assert_eq!(res.source_domain(), "example.net");
    }

    #[test]
    fn test_news_resource_from_article() {
        let article = NewscatcherArticle::default();

        let resource: GenericArticle = article.clone().try_into().unwrap();

        assert_eq!(article.title, resource.title);
        assert_eq!(article.excerpt, resource.snippet);
        assert_eq!(article.link, resource.url.to_string());
        assert_eq!(article.source_domain, resource.source_domain());
        assert_eq!(article.media, resource.image.unwrap().to_string());
        assert_eq!(article.country, resource.country);
        assert_eq!(article.language, resource.language);
        assert_eq!(article.score, resource.score);
        assert_eq!(article.rank, resource.rank.0);
        assert_eq!(article.topic, resource.topic);
        assert_eq!(article.date_published, resource.date_published);
    }

    #[test]
    fn test_news_resource_from_article_invalid_link() {
        let invalid_url = NewscatcherArticle {
            link: String::new(),
            ..NewscatcherArticle::default()
        };

        let res: Result<GenericArticle, _> = invalid_url.try_into();
        assert!(matches!(res, Err(Error::InvalidUrl(_))));
    }

    #[test]
    fn test_news_resource_from_article_empty_media() {
        let article = NewscatcherArticle {
            media: "".to_string(),
            ..NewscatcherArticle::default()
        };

        let res: GenericArticle = article.try_into().unwrap();
        assert!(res.image.is_none());
    }

    #[test]
    fn test_news_resource_from_article_invalid_media() {
        let invalid_url = NewscatcherArticle {
            media: "invalid".to_string(),
            ..NewscatcherArticle::default()
        };

        let res: Result<GenericArticle, _> = invalid_url.try_into();
        assert!(matches!(res, Err(Error::InvalidUrl(_))));
    }
}
