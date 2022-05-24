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

use chrono::NaiveDateTime;
use derive_more::Display;
use uuid::Uuid;

use crate::embedding::Embedding;

/// A unique identifier of a document.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Display)]
pub struct DocumentId(Uuid);

impl From<Uuid> for DocumentId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

#[cfg(test)]
impl DocumentId {
    /// Creates a `DocumentId` from a 128bit value in big-endian order.
    pub const fn from_u128(id: u128) -> Self {
        DocumentId(Uuid::from_u128(id))
    }
}

/// The various kinds of user feedback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UserFeedback {
    /// The user considers this as relevant.
    Relevant,
    /// The user considers this as irrelevant.
    Irrelevant,
    /// The user doesn't give feedback.
    NotGiven,
}

/// Common document properties.
pub trait Document {
    /// Gets the document id.
    fn id(&self) -> DocumentId;

    /// Gets the `SMBert` embedding of the document.
    fn smbert_embedding(&self) -> &Embedding;

    /// Gets the publishing date.
    fn date_published(&self) -> NaiveDateTime;
}

#[cfg(test)]
pub(super) struct TestDocument {
    pub(super) id: DocumentId,
    pub(super) smbert_embedding: Embedding,
    pub(super) date_published: NaiveDateTime,
}

#[cfg(test)]
impl TestDocument {
    pub(super) fn new(id: u128, embedding: impl Into<Embedding>, date_published: &str) -> Self {
        Self {
            id: DocumentId::from_u128(id),
            smbert_embedding: embedding.into(),
            date_published: NaiveDateTime::parse_from_str(date_published, "%Y-%m-%d %H:%M:%S")
                .unwrap(),
        }
    }
}

#[cfg(test)]
impl Document for TestDocument {
    fn id(&self) -> DocumentId {
        self.id
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }

    fn date_published(&self) -> NaiveDateTime {
        self.date_published
    }
}
