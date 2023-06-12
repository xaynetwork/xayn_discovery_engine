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

use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
};

use chrono::{DateTime, TimeZone, Utc};
use derive_more::{Deref, DerefMut, Display, From, Into};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use sqlx::{
    postgres::{PgHasArrayType, PgTypeInfo},
    FromRow,
    Type,
};
use xayn_ai_bert::{NormalizedEmbedding, NormalizedSparseEmbedding};
use xayn_ai_coi::Document as AiDocument;

use crate::error::common::{
    InvalidDocumentId,
    InvalidDocumentPropertyId,
    InvalidDocumentTag,
    InvalidUserId,
};

macro_rules! id_wrapper {
    ($($visibility:vis $name:ident, $error:ident, $is_valid:expr);* $(;)?) => {
        $(
            /// A unique identifier.
            #[derive(
                Deref,
                Into,
                Clone,
                Debug,
                Display,
                PartialEq,
                Eq,
                Hash,
                Serialize,
                Deserialize,
                Type,
                FromRow,
            )]
            #[serde(transparent)]
            #[sqlx(transparent)]
            $visibility struct $name(String);

            impl $name {
                $visibility fn new(value: impl Into<String>) -> Result<Self, $error> {
                    let value = value.into();
                    if $is_valid(&value) {
                        Ok(Self(value))
                    } else {
                        Err($error { value })
                    }
                }
            }

            impl TryFrom<String> for $name {
                type Error = $error;

                fn try_from(value: String) -> Result<Self, Self::Error> {
                    Self::new(value)
                }
            }

            impl TryFrom<&str> for $name {
                type Error = $error;

                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    Self::new(value)
                }
            }

            impl PgHasArrayType for $name {
                fn array_type_info() -> PgTypeInfo {
                    <String as PgHasArrayType>::array_type_info()
                }
            }

            impl Borrow<str> for $name {
                fn borrow(&self) -> &str {
                    self.as_ref()
                }
            }
        )*
    };
}

fn is_valid_id(value: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_\-:@.]+$").unwrap());

    (1..=256).contains(&value.len()) && RE.is_match(value)
}

fn is_valid_tag(value: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\x00]+$").unwrap());

    (1..=256).contains(&value.len()) && RE.is_match(value)
}

id_wrapper! {
    pub(crate) DocumentId, InvalidDocumentId, is_valid_id;
    pub(crate) DocumentPropertyId, InvalidDocumentPropertyId, is_valid_id;
    pub(crate) UserId, InvalidUserId, is_valid_id;
    pub(crate) DocumentTag, InvalidDocumentTag, is_valid_tag;
}

impl DocumentPropertyId {
    pub(crate) const PUBLICATION_DATE: &str = "publication_date";
}

#[derive(Clone, Debug, Deref, Deserialize, From, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct DocumentProperty(Value);

/// Arbitrary properties that can be attached to a document.
// pub(crate) type DocumentProperties = HashMap<DocumentPropertyId, DocumentProperty>;
#[derive(
    Clone, Debug, Default, Deref, DerefMut, Deserialize, From, FromRow, PartialEq, Serialize,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct DocumentProperties(HashMap<DocumentPropertyId, DocumentProperty>);

impl FromIterator<(DocumentPropertyId, DocumentProperty)> for DocumentProperties {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (DocumentPropertyId, DocumentProperty)>,
    {
        iter.into_iter().collect::<HashMap<_, _>>().into()
    }
}

impl DocumentProperties {
    pub(crate) fn serialize_time_to_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.iter()
            .map(|(id, property)| {
                if id.as_str() == DocumentPropertyId::PUBLICATION_DATE {
                    if let Some(publication_date) = property.as_str().and_then(|publication_date| {
                        DateTime::parse_from_rfc3339(publication_date).ok()
                    }) {
                        return (id, Cow::Owned(json!(publication_date.timestamp()).into()));
                    }
                }

                (id, Cow::Borrowed(property))
            })
            .collect::<HashMap<_, _>>()
            .serialize(serializer)
    }

    pub(crate) fn deserialize_time_from_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut properties = Self::deserialize(deserializer)?;
        if let Some(publication_date) = properties.get_mut(DocumentPropertyId::PUBLICATION_DATE) {
            if let Some(date) = publication_date
                .as_i64()
                .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
            {
                publication_date.0 = json!(date.to_rfc3339());
            }
        }

        Ok(properties)
    }
}

/// Represents a result from an interaction query.
#[derive(Clone, Debug)]
pub(crate) struct InteractedDocument {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Embedding from smbert.
    pub(crate) embedding: NormalizedEmbedding,

    /// The tags associated to the document.
    pub(crate) tags: Vec<DocumentTag>,
}

/// Represents a result from a personalization query.
#[derive(Clone, Debug)]
pub(crate) struct PersonalizedDocument {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Similarity score of the personalized document.
    pub(crate) score: f32,

    /// Embedding from smbert.
    pub(crate) embedding: NormalizedEmbedding,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,

    /// The tags associated to the document.
    pub(crate) tags: Vec<DocumentTag>,
}

impl AiDocument for PersonalizedDocument {
    type Id = DocumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn embedding(&self) -> &NormalizedEmbedding {
        &self.embedding
    }
}

/// Represents a document sent for ingestion.
#[derive(Clone, Debug)]
pub(crate) struct IngestedDocument {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Snippet used to calculate embeddings for a document.
    pub(crate) snippet: String,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,

    /// The tags associated to the document.
    pub(crate) tags: Vec<DocumentTag>,

    /// Embedding of the document.
    pub(crate) embedding: NormalizedEmbedding,

    /// Sparse embedding of the document.
    pub(crate) sparse_embedding: NormalizedSparseEmbedding,

    /// Indicates if the document is considered for recommendations.
    pub(crate) is_candidate: bool,
}

#[derive(Debug)]
pub(crate) struct ExcerptedDocument {
    pub(crate) id: DocumentId,
    pub(crate) snippet: String,
    pub(crate) properties: DocumentProperties,
    pub(crate) tags: Vec<DocumentTag>,
    pub(crate) is_candidate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id() {
        assert!(DocumentId::new("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentId::new("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentId::new("0123456789").is_ok());
        assert!(DocumentId::new("_-:@.").is_ok());
        assert!(DocumentId::new("").is_err());
        assert!(DocumentId::new(["a"; 257].join("")).is_err());
        assert!(DocumentId::new("!?ß").is_err());
    }

    #[test]
    fn test_tag() {
        assert!(DocumentTag::new("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentTag::new("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentTag::new("0123456789").is_ok());
        assert!(DocumentTag::new(" .:,;-_#'+*^°!\"§$%&/()=?\\´`@€").is_ok());
        assert!(DocumentTag::new("").is_err());
        assert!(DocumentTag::new(["a"; 257].join("")).is_err());
        assert!(DocumentTag::new("\0").is_err());
    }
}
