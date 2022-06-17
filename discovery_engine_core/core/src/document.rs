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

use std::time::Duration;

use chrono::NaiveDateTime;
use derivative::Derivative;
use derive_more::Display;
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

use xayn_discovery_engine_providers::{GenericArticle, Market, TrendingTopic as BingTopic};

use crate::stack::Id as StackId;

pub use xayn_discovery_engine_ai::Embedding;

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
#[repr(transparent)]
#[cfg_attr(test, derive(Default))]
pub struct Id(Uuid);

impl Id {
    // forbid inline to avoid miscompilation
    #[inline(never)]
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl From<Uuid> for Id {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<Id> for Uuid {
    fn from(id: Id) -> Self {
        id.0
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

impl TryFrom<(GenericArticle, StackId, Embedding)> for Document {
    type Error = Error;

    fn try_from(
        (article, stack_id, smbert_embedding): (GenericArticle, StackId, Embedding),
    ) -> Result<Self, Self::Error> {
        article.try_into().map(|resource| Self {
            id: Id::new(),
            stack_id,
            smbert_embedding,
            resource,
        })
    }
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

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub source_domain: String,

    /// Publishing date.
    pub date_published: NaiveDateTime,

    /// Image attached to the news.
    pub image: Option<Url>,

    /// The rank of the domain of the source,
    pub rank: u64,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// Main topic of the publisher.
    pub topic: String,
}

impl TryFrom<GenericArticle> for NewsResource {
    type Error = Error;
    fn try_from(content: GenericArticle) -> Result<Self, Self::Error> {
        let source_domain = content.source_domain();
        let rank = content.rank();

        Ok(Self {
            title: content.title,
            snippet: content.snippet,
            date_published: content.date_published,
            url: content.url.inner(),
            source_domain,
            image: content.image,
            rank,
            score: content.score,
            country: content.country,
            language: content.language,
            topic: content.topic,
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

/// Log the time that has been spent on the document.
pub struct TimeSpent {
    /// Id of the document.
    pub id: Id,

    /// Precomputed S-mBert of the document.
    pub smbert_embedding: Embedding,

    /// Time spent on the documents in seconds.
    pub time: Duration,
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

    /// Text title of the document.
    pub title: String,

    /// Text snippet of the document.
    pub snippet: String,

    /// Precomputed S-mBert of the document.
    pub smbert_embedding: Embedding,

    /// Reaction.
    pub reaction: UserReaction,

    /// Market from which the document is.
    pub market: Market,
}

/// Represents a [`Document`] in the document history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricDocument {
    /// See  [`Document::id`].
    pub id: Id,
    /// See [`NewsResource::url`].
    pub url: Url,
    /// See [`NewsResource::snippet`].
    pub snippet: String,
    /// See [`NewsResource::title`].
    pub title: String,
}

/// A trending topic.
pub struct TrendingTopic {
    /// Id of the topic.
    pub id: Id,
    /// Precomputed S-mBert of the topic.
    pub smbert_embedding: Embedding,
    /// Title of the topic.
    pub name: String,
    /// Query term that returns this topic.
    pub query: String,
    /// Link to a related image.
    pub image: Option<Url>,
}

impl TryFrom<(BingTopic, Embedding)> for TrendingTopic {
    type Error = Error;
    fn try_from((topic, smbert_embedding): (BingTopic, Embedding)) -> Result<Self, Self::Error> {
        let url = topic.image.url;
        let image = (!url.is_empty()).then(|| Url::parse(&url)).transpose()?;

        Ok(Self {
            id: Id::new(),
            smbert_embedding,
            name: topic.name,
            query: topic.query.text,
            image,
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use xayn_discovery_engine_providers::NewscatcherArticle;

    use super::*;

    impl Default for NewsResource {
        fn default() -> Self {
            Self {
                title: String::default(),
                snippet: String::default(),
                url: example_url(),
                source_domain: "example.com".to_string(),
                image: None,
                date_published: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
                score: None,
                rank: 0,
                country: "en".to_string(),
                language: "en".to_string(),
                topic: "news".to_string(),
            }
        }
    }

    fn example_url() -> Url {
        Url::parse("https://example.net").unwrap(/* used only in tests */)
    }

    fn mock_newscatcher_article() -> NewscatcherArticle {
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

    fn mock_generic_article() -> GenericArticle {
        mock_newscatcher_article().try_into().unwrap()
    }

    impl TryFrom<GenericArticle> for HistoricDocument {
        type Error = Error;

        fn try_from(article: GenericArticle) -> Result<Self, Self::Error> {
            Ok(Self {
                id: Uuid::new_v4().into(),
                url: article.url.inner(),
                snippet: article.snippet,
                title: article.title,
            })
        }
    }

    #[test]
    fn test_news_resource_from_article() {
        let article = mock_generic_article();

        let resource: NewsResource = article.clone().try_into().unwrap();

        let rank = article.rank();

        assert_eq!(article.title, resource.title);
        assert_eq!(article.snippet, resource.snippet);
        assert_eq!(article.url.to_string(), resource.url.to_string());
        assert_eq!(article.source_domain(), resource.source_domain);
        assert_eq!(
            article.image.unwrap().to_string(),
            resource.image.unwrap().to_string()
        );
        assert_eq!(article.country, resource.country);
        assert_eq!(article.language, resource.language);
        assert_eq!(article.score, resource.score);
        assert_eq!(rank, resource.rank);
        assert_eq!(article.topic, resource.topic);
        assert_eq!(article.date_published, resource.date_published);
    }
}
