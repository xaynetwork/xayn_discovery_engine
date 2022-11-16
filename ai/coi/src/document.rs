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

use crate::embedding::Embedding;

/// Common document properties.
pub trait Document {
    type Id: std::fmt::Display + std::fmt::Debug + Eq + std::hash::Hash + Clone;

    /// Gets the document id.
    fn id(&self) -> &Self::Id;

    /// Gets the `Bert` embedding of the document.
    fn bert_embedding(&self) -> &Embedding;
}

#[cfg(test)]
pub(crate) mod tests {
    use derive_more::Display;
    use uuid::Uuid;

    use super::*;

    /// A unique identifier of a document.
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Display)]
    pub(crate) struct DocumentId(Uuid);

    impl From<Uuid> for DocumentId {
        fn from(id: Uuid) -> Self {
            Self(id)
        }
    }

    impl DocumentId {
        /// Creates a `DocumentId` from a 128bit value in big-endian order.
        pub(crate) const fn from_u128(id: u128) -> Self {
            DocumentId(Uuid::from_u128(id))
        }
    }

    pub(crate) struct TestDocument {
        pub(crate) id: DocumentId,
        pub(crate) bert_embedding: Embedding,
    }

    impl TestDocument {
        pub(crate) fn new(id: u128, embedding: impl Into<Embedding>) -> Self {
            Self {
                id: DocumentId::from_u128(id),
                bert_embedding: embedding.into(),
            }
        }
    }

    impl Document for TestDocument {
        type Id = DocumentId;

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn bert_embedding(&self) -> &Embedding {
            &self.bert_embedding
        }
    }
}
