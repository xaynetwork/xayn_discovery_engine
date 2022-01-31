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

use std::{convert::TryFrom, time::Duration};

use derivative::Derivative;
use derive_more::Display;
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use xayn_ai::ranker::Embedding;

use crate::stack::Id as StackId;

/// Errors that could happen when constructing a [`Document`].
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Failed to parse Uuid: {0}.
    Parse(#[from] uuid::Error),
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

    /// Position of the document from the source.
    pub rank: usize,

    /// Text title of the document.
    pub title: String,

    /// Text snippet of the document.
    pub snippet: String,

    /// URL of the document.
    pub url: String,

    /// Domain of the document.
    pub domain: String,

    /// Embedding from smbert.
    pub smbert_embedding: Embedding,
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
