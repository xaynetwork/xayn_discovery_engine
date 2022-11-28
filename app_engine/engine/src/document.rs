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

use chrono::{offset::Utc, DateTime};
use derivative::Derivative;
use derive_more::Display;
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use thiserror::Error;
use url::Url;
use uuid::Uuid;
use xayn_ai_coi::Document as AiDocument;
pub use xayn_ai_coi::Embedding;
use xayn_discovery_engine_providers::GenericArticle;

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
#[derive(
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
    Hash,
    Serialize,
    Deserialize,
    Display,
    sqlx::Type,
    sqlx::FromRow,
)]
#[repr(transparent)]
#[sqlx(transparent)]
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

    /// Embedding from Bert.
    pub bert_embedding: Embedding,

    /// Reaction.
    pub reaction: Option<UserReaction>,

    /// Resource this document refers to.
    pub resource: NewsResource,
}

impl TryFrom<(GenericArticle, StackId, Embedding)> for Document {
    type Error = Error;

    fn try_from(
        (article, stack_id, bert_embedding): (GenericArticle, StackId, Embedding),
    ) -> Result<Self, Self::Error> {
        let resource: NewsResource = article.into();
        Ok(Self {
            id: Id::new(),
            stack_id,
            bert_embedding,
            resource,
            reaction: None,
        })
    }
}

impl AiDocument for Document {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn bert_embedding(&self) -> &Embedding {
        &self.bert_embedding
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
    pub date_published: DateTime<Utc>,

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

impl NewsResource {
    /// Returns the title, or if empty the snippet instead.
    pub fn title_or_snippet(&self) -> &str {
        if self.title.is_empty() {
            &self.snippet
        } else {
            &self.title
        }
    }

    /// Returns the snippet, or if empty the title instead.
    pub fn snippet_or_title(&self) -> &str {
        if self.snippet.is_empty() {
            &self.title
        } else {
            &self.snippet
        }
    }
}

impl From<GenericArticle> for NewsResource {
    fn from(article: GenericArticle) -> Self {
        let source_domain = article.source_domain();
        let rank = article.rank.0;
        Self {
            title: article.title,
            snippet: article.snippet,
            date_published: article.date_published,
            url: article.url.to_inner(),
            source_domain,
            image: article.image,
            rank,
            score: article.score,
            country: article.country,
            language: article.language,
            topic: article.topic,
        }
    }
}

/// Indicates user's "sentiment" towards the document,
/// essentially if the user "liked" or "disliked" the document.
#[derive(
    Clone,
    Copy,
    Debug,
    Derivative,
    Eq,
    PartialEq,
    Serialize_repr,
    Deserialize_repr,
    num_derive::FromPrimitive,
)]
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

    /// Time spent on the documents in seconds.
    pub view_time: Duration,

    /// The way the document was viewed.
    pub view_mode: ViewMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, num_derive::FromPrimitive)]
#[repr(u32)]
pub enum ViewMode {
    Story = 0,
    Reader = 1,
    Web = 2,
}

/// User reacted to a document.
#[derive(Debug)]
pub struct UserReacted {
    /// Id of the document.
    pub id: Id,

    /// Reaction.
    pub reaction: UserReaction,
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

/// A source domain with an associated weight.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct WeightedSource {
    /// Source domain.
    pub source: String,
    /// Weight of the source in terms of user reactions.
    pub weight: i32,
}

#[cfg(test)]
pub(crate) mod tests {
    use chrono::TimeZone;
    use xayn_discovery_engine_providers::{Rank, UrlWithDomain};

    use super::*;

    impl Default for NewsResource {
        fn default() -> Self {
            Self {
                title: String::default(),
                snippet: String::default(),
                url: example_url(),
                source_domain: "example.com".to_string(),
                image: None,
                date_published: Utc.with_ymd_and_hms(2022, 1, 1, 9, 0, 0).unwrap(),
                score: None,
                rank: 0,
                country: "GB".to_string(),
                language: "en".to_string(),
                topic: "news".to_string(),
            }
        }
    }

    fn example_url() -> Url {
        Url::parse("https://example.net").unwrap(/* used only in tests */)
    }

    pub(crate) fn mock_generic_article() -> GenericArticle {
        GenericArticle {
            title: "title".to_string(),
            score: Some(0.75),
            rank: Rank::new(10),
            snippet: "summary of the article".to_string(),
            topic: "news".to_string(),
            country: "GB".to_string(),
            language: "en".to_string(),
            date_published: Utc.with_ymd_and_hms(2022, 1, 1, 9, 0, 0).unwrap(),
            url: UrlWithDomain::new(Url::parse("https://example.com/news/").unwrap()).unwrap(),
            image: Some(Url::parse("https://example.com/news/image.jpg").unwrap()),
            embedding: None,
        }
    }

    impl From<GenericArticle> for HistoricDocument {
        fn from(article: GenericArticle) -> Self {
            Self {
                id: Uuid::new_v4().into(),
                url: article.url.to_inner(),
                snippet: article.snippet,
                title: article.title,
            }
        }
    }

    #[test]
    fn test_news_resource_from_article() {
        let article = mock_generic_article();

        let resource: NewsResource = article.clone().try_into().unwrap();

        let rank = article.rank.0;

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