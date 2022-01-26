// Copyright 2021 Xayn AG
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

//! Personalized document that is returned from [`Engine`](crate::engine::Engine).

use std::{
    convert::{TryFrom, TryInto},
    time::Duration,
};

use chrono::NaiveDateTime;
use derivative::Derivative;
use derive_more::Display;
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use xayn_ai::ranker::Embedding;

use xayn_discovery_engine_providers::{Article, Topic};

use crate::stack::Id as StackId;

/// Errors that could happen when constructing a [`Document`].
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Failed to parse Uuid: {0}.
    Parse(#[from] uuid::Error),

    /// Impossible to parse the provided url: {0}.
    InvalidUrl(#[from] url::ParseError),
}

/// Unique identifier of the [`Document`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
#[cfg_attr(test, derive(Default))]
pub struct Id(pub Uuid);

impl Id {
    /// Creates a [`Id`] from a 128bit value in big-endian order.
    pub fn from_u128(id: u128) -> Self {
        Id(Uuid::from_u128(id))
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl TryFrom<&[u8]> for Id {
    type Error = Error;

    fn try_from(id: &[u8]) -> Result<Self, Self::Error> {
        Ok(Id(Uuid::from_slice(id)?))
    }
}

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(Default))]
pub struct Document {
    /// Unique identifier of the document.
    pub id: Id,

    /// Stack from which the document has been taken.
    pub stack_id: StackId,

    /// Embedding from smbert.
    pub smbert_embedding: Embedding,

    /// Resource this document refers to.
    pub web_resource: WebResource,
}

/// Represents different kinds of resources like web, image, video, news,
/// that are delivered by an external content API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResource {
    /// Title of the resource.
    pub title: String,

    /// Snippet of the resource.
    pub snippet: String,

    /// Url to reach the resource.
    pub url: Url,

    /// Url to display to the user.
    // TODO: not sure it make sense to have this here, seems a ui decision
    pub display_url: Url,

    /// Publishing date.
    pub date_published: NaiveDateTime,

    /// Provider.
    pub provider: Option<WebResourceProvider>,

    /// The rank of the domain of the source,
    pub rank: usize,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// Main topic of the publisher.
    pub topic: Topic,
}

impl TryFrom<Article> for WebResource {
    type Error = Error;
    fn try_from(article: Article) -> Result<Self, Self::Error> {
        let media = article.media;

        Ok(WebResource {
            title: article.title,
            snippet: article.excerpt,
            date_published: article.published_date,
            url: Url::parse(&article.link)?,
            display_url: Url::parse(&article.clean_url)?,
            rank: article.rank,
            score: article.score,
            country: article.country,
            language: article.language,
            topic: article.topic,
            provider: Some(WebResourceProvider {
                name: String::new(),
                thumbnail: (!media.is_empty())
                    .then(|| Url::parse(&media))
                    .transpose()?,
            }),
        })
    }
}

/// Represents the provider of a [`WebResource`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResourceProvider {
    /// Provider's name.
    pub name: String,

    /// Url to the thumbnail-sized logo for the provider.
    pub thumbnail: Option<Url>,
}

/// Indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
#[derive(Clone, Copy, Debug, Derivative, Serialize, Deserialize)]
#[derivative(Default)]
pub enum UserReaction {
    /// No reaction from the user.
    #[derivative(Default)]
    Neutral,

    /// The user is interested.
    Positive,

    /// The user is not interested.
    Negative,
}

/// Log the time that has been spent on the document.
pub struct TimeSpent {
    /// Id of the document.
    pub id: Id,

    /// Precomputed S-mBert of the document.
    pub smbert: Embedding,

    /// Time spent on the documents in seconds.
    pub seconds: Duration,
    /* we don't have a `DocumentViewMode` in here because at the moment the
       coi just consider one time. On the dart side we are saving all these values
       and when we call the feedbackloop we will decide which value to use or to aggregate them.
    */
    /// Reaction.
    pub reaction: UserReaction,
}

/// User reacted to a document.
pub struct UserReacted {
    /// Id of the document.
    pub id: Id,

    /// Stack from which the document has been taken.
    pub stack_id: StackId,

    /// Text snippet of the document.
    pub snippet: String,

    /// Precomputed S-mBert of the document.
    pub smbert: Embedding,

    /// Reaction.
    pub reaction: UserReaction,
}

#[allow(dead_code)]
pub(crate) fn document_from_article(
    article: Article,
    stack_id: StackId,
    smbert_embedding: Embedding,
) -> Result<Document, Error> {
    Ok(Document {
        id: Uuid::new_v4().into(),
        stack_id,
        smbert_embedding,
        web_resource: article.try_into()?,
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use claim::{assert_matches, assert_none};

    use super::*;

    fn example_url() -> Url {
        Url::parse("https://example.net").unwrap(/* used only in tests */)
    }

    impl Default for WebResource {
        fn default() -> Self {
            Self {
                title: String::default(),
                snippet: String::default(),
                url: example_url(),
                display_url: example_url(),
                date_published: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
                provider: None,
                score: None,
                rank: 0,
                country: "en".to_string(),
                language: "en".to_string(),
                topic: Topic::Unrecognized,
            }
        }
    }

    impl Default for WebResourceProvider {
        fn default() -> Self {
            Self {
                name: String::default(),
                thumbnail: None,
            }
        }
    }

    #[test]
    fn test_web_resource_from_article() {
        let title = "title".to_string();
        let clean_url = "https://example.com/".to_string();
        let excerpt = "summary of the article".to_string();
        let link = "https://example.com/news/".to_string();
        let media = "https://example.com/news/image/".to_string();
        let country = "en".to_string();
        let language = "en".to_string();
        let score = Some(0.75);
        let rank = 10;
        let topic = Topic::News;
        let published_date = NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0);

        let article = Article {
            id: "id".to_string(),
            title: title.clone(),
            score,
            rank,
            clean_url: clean_url.clone(),
            excerpt: excerpt.clone(),
            link: link.clone(),
            media: media.clone(),
            topic: topic.clone(),
            country: country.clone(),
            language: language.clone(),
            published_date,
        };

        let web_resource: WebResource = article.try_into().unwrap();

        assert_eq!(title, web_resource.title);
        assert_eq!(clean_url, web_resource.display_url.to_string());
        assert_eq!(excerpt, web_resource.snippet);
        assert_eq!(link, web_resource.url.to_string());
        assert_eq!(country, web_resource.country);
        assert_eq!(language, web_resource.language);
        assert_eq!(score, web_resource.score);
        assert_eq!(rank, web_resource.rank);
        assert_eq!(topic, web_resource.topic);
        assert_eq!(published_date, web_resource.date_published);

        let provider = web_resource.provider.unwrap();

        assert_eq!(String::new(), provider.name);
        assert_eq!(media, provider.thumbnail.unwrap().to_string());
    }

    #[test]
    fn test_web_resource_from_article_invalid_links() {
        let article = Article {
            id: "id".to_string(),
            title: "title".to_string(),
            score: Some(0.75),
            rank: 10,
            clean_url: "https://example.com/".to_string(),
            excerpt: "summary of the article".to_string(),
            link: "https://example.com/news/".to_string(),
            media: "https://example.com/news/image/".to_string(),
            topic: Topic::News,
            country: "en".to_string(),
            language: "en".to_string(),
            published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
        };

        let invalid_url = Article {
            link: String::new(),
            ..article.clone()
        };
        let res: Result<WebResource, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));

        let invalid_url = Article {
            clean_url: String::new(),
            ..article.clone()
        };
        let res: Result<WebResource, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));

        let invalid_url = Article {
            media: "invalid".to_string(),
            ..article
        };
        let res: Result<WebResource, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));
    }

    #[test]
    fn test_web_resource_from_article_empty_media() {
        let article = Article {
            id: "id".to_string(),
            title: "title".to_string(),
            score: Some(0.75),
            rank: 10,
            clean_url: "https://example.com/".to_string(),
            excerpt: "summary of the article".to_string(),
            link: "https://example.com/news/".to_string(),
            media: "".to_string(),
            topic: Topic::News,
            country: "en".to_string(),
            language: "en".to_string(),
            published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
        };

        let web_resource: WebResource = article.try_into().unwrap();
        let thumbnail = web_resource.provider.unwrap().thumbnail;
        assert_none!(thumbnail);
    }
}
