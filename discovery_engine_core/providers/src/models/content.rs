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

use crate::{Error, NewscatcherArticle};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use url::Url;

/// A helper type used to ensure that, within the [`GenericArticle`] struct,
/// we never use a URL which does not have a domain, such as `file:///foo/bar`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UrlWithDomain(Url);

impl Deref for UrlWithDomain {
    type Target = Url;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UrlWithDomain {
    pub fn new(url: Url) -> Option<Self> {
        if url.domain().is_none() {
            None
        } else {
            Some(UrlWithDomain(url))
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn domain(&self) -> &str {
        self.0.domain().unwrap(/* constructor makes sure we have a domain */)
    }

    pub fn inner(&self) -> Url {
        self.0.clone()
    }
}

impl TryFrom<&str> for UrlWithDomain {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(UrlWithDomain::new(Url::parse(value).unwrap()).unwrap())
    }
}

/// Represents a news that is delivered by an external content API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericArticle {
    pub title: String,
    pub snippet: String,
    pub url: UrlWithDomain,
    pub date_published: NaiveDateTime,
    pub country: String,
    pub language: String,
    pub topic: String,
    pub image: Option<Url>,

    /// Private so that we can centrally control the default
    /// value of `rank`
    rank: Option<u64>,

    /// How much the article match the query.
    pub score: Option<f32>,
}

impl GenericArticle {
    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub fn source_domain(&self) -> String {
        self.url.domain().to_string()
    }

    /// The rank of the domain of the source
    pub fn rank(&self) -> u64 {
        // TODO: What should the default value here be?
        self.rank.unwrap_or(0)
    }

    pub fn set_rank(&mut self, rank: u64) {
        self.rank = Some(rank);
    }

    /// Gets the excerpt or falls back to the title if the excerpt is empty.
    pub fn excerpt_or_title(&self) -> &str {
        (!self.snippet.is_empty())
            .then(|| &self.snippet)
            .unwrap_or(&self.title)
    }
}

impl TryFrom<NewscatcherArticle> for GenericArticle {
    type Error = Error;
    fn try_from(article: NewscatcherArticle) -> Result<Self, Self::Error> {
        let media = article.media;
        let image = (!media.is_empty())
            .then(|| Url::parse(&media))
            .transpose()?;

        let url = Url::parse(&article.link)?;
        let url = UrlWithDomain::new(url)
            .ok_or_else(|| Error::MissingDomainInUrl(article.link.clone()))?;

        Ok(Self {
            title: article.title,
            snippet: article.excerpt,
            date_published: article.published_date,
            url,
            image,
            rank: Some(article.rank),
            score: article.score,
            country: article.country,
            language: article.language,
            topic: article.topic,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use claim::{assert_matches, assert_none};

    use super::*;

    fn mock_resource() -> GenericArticle {
        GenericArticle {
            title: String::default(),
            snippet: String::default(),
            url: example_url(),
            image: None,
            date_published: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
            score: None,
            rank: Some(0),
            country: "en".to_string(),
            language: "en".to_string(),
            topic: "news".to_string(),
        }
    }

    fn example_url() -> UrlWithDomain {
        let url = Url::parse("https://example.net").unwrap(/* used only in tests */);
        UrlWithDomain::new(url).unwrap(/* used only in tests */)
    }

    fn mock_article() -> NewscatcherArticle {
        NewscatcherArticle {
            title: "title".to_string(),
            score: Some(0.75),
            rank: 10,
            source_domain: "example.com".to_string(),
            excerpt: "summary of the article".to_string(),
            link: "https://example.com/news/".to_string(),
            media: "https://example.com/news/image/".to_string(),
            topic: "news".to_string(),
            country: "EN".to_string(),
            language: "en".to_string(),
            published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
        }
    }

    #[test]
    fn test_url_with_domain() {
        let url = Url::parse("file:///foo/bar").unwrap();
        let wrapped = UrlWithDomain::new(url);
        assert!(wrapped.is_none());
    }

    #[test]
    fn test_source_domain_extraction() {
        let res = mock_resource();
        assert_eq!(res.source_domain(), "example.net");
    }

    #[test]
    fn test_news_resource_from_article() {
        let article = mock_article();

        let resource: GenericArticle = article.clone().try_into().unwrap();

        assert_eq!(article.title, resource.title);
        assert_eq!(article.excerpt, resource.snippet);
        assert_eq!(article.link, resource.url.to_string());
        assert_eq!(article.source_domain, resource.source_domain());
        assert_eq!(article.media, resource.image.unwrap().to_string());
        assert_eq!(article.country, resource.country);
        assert_eq!(article.language, resource.language);
        assert_eq!(article.score, resource.score);
        assert_eq!(article.rank, resource.rank.unwrap());
        assert_eq!(article.topic, resource.topic);
        assert_eq!(article.published_date, resource.date_published);
    }

    #[test]
    fn test_news_resource_from_article_invalid_link() {
        let invalid_url = NewscatcherArticle {
            link: String::new(),
            ..mock_article()
        };

        let res: Result<GenericArticle, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));
    }

    #[test]
    fn test_news_resource_from_article_empty_media() {
        let article = NewscatcherArticle {
            media: "".to_string(),
            ..mock_article()
        };

        let res: GenericArticle = article.try_into().unwrap();
        assert_none!(res.image);
    }

    #[test]
    fn test_news_resource_from_article_invalid_media() {
        let invalid_url = NewscatcherArticle {
            media: "invalid".to_string(),
            ..mock_article()
        };

        let res: Result<GenericArticle, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));
    }
}
