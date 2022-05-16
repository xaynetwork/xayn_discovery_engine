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

use derive_more::Display;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryFrom;
use uuid::Uuid;

use crate::Error;

/// A unique identifier of a document.
#[repr(transparent)]
#[cfg_attr(test, derive(Default))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
#[serde(transparent)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    /// Creates a `DocumentId` from a 128bit value in big-endian order.
    pub fn from_u128(id: u128) -> Self {
        DocumentId(Uuid::from_u128(id))
    }
}

impl TryFrom<&str> for DocumentId {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Ok(DocumentId(Uuid::parse_str(id)?))
    }
}

/// A unique identifier of a session.
#[repr(transparent)]
#[cfg_attr(test, derive(Default))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// New identifier from a 128bit value in big-endian order.
    pub fn from_u128(id: u128) -> Self {
        Self(Uuid::from_u128(id))
    }
}

impl TryFrom<&str> for SessionId {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Ok(Self(Uuid::parse_str(id)?))
    }
}

/// A unique identifier of a query.
#[repr(transparent)]
#[cfg_attr(test, derive(Default))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize, Display)]
#[serde(transparent)]
pub struct QueryId(pub Uuid);

impl QueryId {
    /// New identifier from a 128bit value in big-endian order.
    pub fn from_u128(id: u128) -> Self {
        Self(Uuid::from_u128(id))
    }
}

impl TryFrom<&str> for QueryId {
    type Error = Error;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        Ok(Self(Uuid::parse_str(id)?))
    }
}

/// Represents a result from a query.
#[cfg_attr(test, derive(Default))]
#[derive(Debug, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier of the document
    pub id: DocumentId,
    /// Position of the document from the source
    pub rank: usize,
    /// Text title of the document
    pub title: String,
    /// Text snippet of the document
    pub snippet: String,
    /// Session of the document
    pub session: SessionId,
    /// Query count within session
    pub query_count: usize,
    /// Query identifier of the document
    pub query_id: QueryId,
    /// Query of the document
    pub query_words: String,
    /// URL of the document
    pub url: String,
    /// Domain of the document
    pub domain: String,
}

/// Represents a historical result from a query.
#[cfg_attr(test, derive(Default))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct DocumentHistory {
    /// Unique identifier of the document
    pub id: DocumentId,
    /// Relevance level of the document
    pub relevance: Relevance,
    /// A flag that indicates whether the user liked the document
    pub user_feedback: UserFeedback,
    /// Session of the document
    pub session: SessionId,
    /// Query count within session
    pub query_count: usize,
    /// Query identifier of the document
    pub query_id: QueryId,
    /// Query of the document
    pub query_words: String,
    /// Day of week query was performed
    pub day: DayOfWeek,
    /// URL of the document
    pub url: String,
    /// Domain of the document
    pub domain: String,
    /// Reranked position of the document
    pub rank: usize,
    /// User interaction for the document
    pub user_action: UserAction,
}

/// The various kinds of user feedback.
#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum UserFeedback {
    /// The user considers this as relevant.
    Relevant = 0,
    /// The user considers this as irrelevant.
    Irrelevant = 1,
    /// The user doesn't give feedback.
    NotGiven = 2,
}

impl Default for UserFeedback {
    fn default() -> Self {
        Self::NotGiven
    }
}

/// The relevance of a document.
#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Relevance {
    /// The document is of low relevance.
    Low = 0,
    /// The document is of medium relevance.
    Medium = 1,
    /// The document is of high relevance.
    High = 2,
}

/// The action of the user on a document.
#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum UserAction {
    /// The user missed the document.
    Miss = 0,
    /// The user skipped the document.
    Skip = 1,
    /// The user clicked the document.
    Click = 2,
}

/// The day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum DayOfWeek {
    /// Monday.
    Mon = 0,
    /// Tuesday.
    Tue = 1,
    /// Wednesday.
    Wed = 2,
    /// Thursday.
    Thu = 3,
    /// Friday.
    Fri = 4,
    /// Saturday.
    Sat = 5,
    /// Sunday.
    Sun = 6,
}

impl DayOfWeek {
    /// Crates a `DayOfWeek` based on a wrap-around offset from `Mon`.
    pub fn from_day_offset(day_offset: usize) -> DayOfWeek {
        static DAYS: &[DayOfWeek] = &[
            DayOfWeek::Mon,
            DayOfWeek::Tue,
            DayOfWeek::Wed,
            DayOfWeek::Thu,
            DayOfWeek::Fri,
            DayOfWeek::Sat,
            DayOfWeek::Sun,
        ];
        DAYS[day_offset % 7]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Default for Relevance {
        fn default() -> Self {
            Self::Low
        }
    }

    impl Default for UserAction {
        fn default() -> Self {
            Self::Miss
        }
    }

    impl Default for DayOfWeek {
        fn default() -> Self {
            DayOfWeek::Mon
        }
    }
}
