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
use serde_repr::{Deserialize_repr, Serialize_repr};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

use xayn_discovery_engine_providers::Article;

use crate::stack::Id as StackId;

pub use xayn_ai::ranker::Embedding;

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
    pub resource: NewsResource,
}

/// Represents a news that is delivered by an external content API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsResource {
    /// Title of the resource.
    pub title: String,

    /// Snippet of the resource.
    pub snippet: String,

    /// Url to reach the resource.
    pub url: Url,

    /// Url to the source of this news.
    pub source_url: Url,

    /// Publishing date.
    pub date_published: NaiveDateTime,

    /// Thumbnail of the image attached to the news.
    pub thumbnail: Option<Url>,

    /// The rank of the domain of the source,
    pub rank: usize,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// Main topic of the publisher.
    pub topic: String,
}

impl TryFrom<Article> for NewsResource {
    type Error = Error;
    fn try_from(article: Article) -> Result<Self, Self::Error> {
        let media = article.media;

        Ok(NewsResource {
            title: article.title,
            snippet: article.excerpt,
            date_published: article.published_date,
            url: Url::parse(&article.link)?,
            source_url: Url::parse(&article.clean_url)?,
            thumbnail: (!media.is_empty())
                .then(|| Url::parse(&media))
                .transpose()?,
            rank: article.rank,
            score: article.score,
            country: article.country,
            language: article.language,
            topic: article.topic.to_string(),
        })
    }
}

/// Indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
#[derive(Clone, Copy, Debug, Derivative, PartialEq, Serialize_repr, Deserialize_repr)]
#[derivative(Default)]
#[repr(u8)]
pub enum UserReaction {
    /// No reaction from the user.
    #[derivative(Default)]
    Neutral = 0,

    /// The user is interested.
    Positive = 1,

    /// The user is not interested.
    Negative = 2,
}

#[derive(Debug, DisplayDoc, Error)]
/// Received an unsupported user reaction, int repr: {reaction}
pub struct UnsupportedUserReaction {
    reaction: u8,
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
        resource: article.try_into()?,
    })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use claim::{assert_matches, assert_none};

    use xayn_discovery_engine_providers::Topic;

    use super::*;

    impl Default for NewsResource {
        fn default() -> Self {
            Self {
                title: String::default(),
                snippet: String::default(),
                url: example_url(),
                source_url: example_url(),
                thumbnail: None,
                date_published: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
                score: None,
                rank: 0,
                country: "en".to_string(),
                language: "en".to_string(),
                topic: Topic::Unrecognized.to_string(),
            }
        }
    }

    fn example_url() -> Url {
        Url::parse("https://example.net").unwrap(/* used only in tests */)
    }

    fn mock_article() -> Article {
        Article {
            id: "id".to_string(),
            title: "title".to_string(),
            score: Some(0.75),
            rank: 10,
            clean_url: "https://example.com/".to_string(),
            excerpt: "summary of the article".to_string(),
            link: "https://example.com/news/".to_string(),
            media: "https://example.com/news/image/".to_string(),
            topic: Topic::News,
            country: "EN".to_string(),
            language: "en".to_string(),
            published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
        }
    }

    #[test]
    fn test_news_resource_from_article() {
        let article = mock_article();

        let resource: NewsResource = article.clone().try_into().unwrap();

        assert_eq!(article.title, resource.title);
        assert_eq!(article.excerpt, resource.snippet);
        assert_eq!(article.link, resource.url.to_string());
        assert_eq!(article.clean_url, resource.source_url.to_string());
        assert_eq!(article.media, resource.thumbnail.unwrap().to_string());
        assert_eq!(article.country, resource.country);
        assert_eq!(article.language, resource.language);
        assert_eq!(article.score, resource.score);
        assert_eq!(article.rank, resource.rank);
        assert_eq!(article.topic.to_string(), resource.topic);
        assert_eq!(article.published_date, resource.date_published);
    }

    #[test]
    fn test_news_resource_from_article_invalid_link() {
        let invalid_url = Article {
            link: String::new(),
            ..mock_article()
        };

        let res: Result<NewsResource, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));
    }

    #[test]
    fn test_news_resource_from_article_empty_media() {
        let article = Article {
            media: "".to_string(),
            ..mock_article()
        };

        let res: NewsResource = article.try_into().unwrap();
        assert_none!(res.thumbnail);
    }

    #[test]
    fn test_news_resource_from_article_invalid_media() {
        let invalid_url = Article {
            media: "invalid".to_string(),
            ..mock_article()
        };

        let res: Result<NewsResource, _> = invalid_url.try_into();
        assert_matches!(res.unwrap_err(), Error::InvalidUrl(_));
    }
}
