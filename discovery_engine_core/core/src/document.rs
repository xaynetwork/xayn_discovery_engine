use crate::error::Error;
use derive_more::Deref;
use ndarray::{Array, Dimension, Ix1};
use std::convert::TryFrom;
use uuid::Uuid;

/// Unique identifier of the document
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct DocumentId(pub Uuid);

impl DocumentId {
    /// Creates a DocumentId from a 128bit value in big-endian order.
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

/// Represents a result from a query.
#[derive(Debug, Clone)]
pub struct Document {
    /// Unique identifier of the document
    pub id: DocumentId,
    /// Position of the document from the source
    pub rank: usize,
    /// Text title of the document
    pub title: String,
    /// Text snippet of the document
    pub snippet: String,
    /// URL of the document
    pub url: String,
    /// Domain of the document
    pub domain: String,
    /// Embedding from smbert
    pub smbert_embedding: Embedding1,
}

/// A d-dimensional sequence embedding.
#[derive(Clone, Debug, Deref)]
pub struct Embedding<D>(Array<f32, D>)
where
    D: Dimension;

/// A 1-dimensional sequence embedding.
pub type Embedding1 = Embedding<Ix1>;
