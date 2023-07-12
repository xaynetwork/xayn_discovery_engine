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

use xayn_ai_bert::NormalizedEmbedding;

/// Common document properties.
pub trait Document {
    type Id: std::fmt::Display + std::fmt::Debug + Eq + std::hash::Hash + Clone;

    /// Gets the document id.
    fn id(&self) -> &Self::Id;

    /// Gets the embedding of the document.
    fn embedding(&self) -> &NormalizedEmbedding;
}

#[cfg(test)]
pub(crate) mod tests {
    use derive_more::Display;
    use uuid::Uuid;
    use xayn_test_utils::uuid::mock_uuid;

    use super::*;

    /// A unique identifier of a document.
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Display)]
    pub(crate) struct DocumentId(Uuid);

    impl DocumentId {
        /// Creates a mocked `Document` id from a mocked UUID, cf. [`mock_uuid()`].
        pub(crate) const fn mocked(id: usize) -> Self {
            DocumentId(mock_uuid(id))
        }
    }

    pub(crate) struct TestDocument {
        pub(crate) id: DocumentId,
        pub(crate) embedding: NormalizedEmbedding,
    }

    impl TestDocument {
        pub(crate) fn new(id: usize, embedding: NormalizedEmbedding) -> Self {
            Self {
                id: DocumentId::mocked(id),
                embedding,
            }
        }
    }

    impl Document for TestDocument {
        type Id = DocumentId;

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn embedding(&self) -> &NormalizedEmbedding {
            &self.embedding
        }
    }
}
