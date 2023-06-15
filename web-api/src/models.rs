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

use std::{borrow::Borrow, collections::HashMap};

use derive_more::{Deref, Display, Into};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgHasArrayType, PgTypeInfo},
    FromRow,
    Type,
};
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::Document as AiDocument;

use crate::error::common::{
    InvalidDocumentId,
    InvalidDocumentPropertyId,
    InvalidDocumentSnippet,
    InvalidDocumentTag,
    InvalidUserId,
};

macro_rules! string_wrapper {
    ($($(#[$attribute:meta])* $visibility:vis $name:ident, $error:ident, $is_valid:expr);* $(;)?) => {
        $(
            $(#[$attribute])*
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

            impl TryFrom<String> for $name {
                type Error = $error;

                fn try_from(mut value: String) -> Result<Self, Self::Error> {
                    let trimmed = value.trim();
                    if trimmed.len() < value.len() {
                        value = trimmed.to_string();
                    }

                    if $is_valid(&value) {
                        Ok(Self(value))
                    } else {
                        Err($error { value })
                    }
                }
            }

            impl TryFrom<&str> for $name {
                type Error = $error;

                fn try_from(value: &str) -> Result<Self, Self::Error> {
                    value.to_string().try_into()
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

fn is_valid_string(value: &str, len: usize) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\x00]+$").unwrap());

    (1..=len).contains(&value.len()) && RE.is_match(value)
}

string_wrapper! {
    /// A unique document identifier.
    pub(crate) DocumentId, InvalidDocumentId, is_valid_id;
    /// A unique document property identifier.
    pub(crate) DocumentPropertyId, InvalidDocumentPropertyId, is_valid_id;
    /// A unique user identifier.
    pub(crate) UserId, InvalidUserId, is_valid_id;
    /// A document snippet.
    pub(crate) DocumentSnippet, InvalidDocumentSnippet, |value| is_valid_string(value, 1024);
    /// A document tag.
    pub(crate) DocumentTag, InvalidDocumentTag, |value| is_valid_string(value, 256);
}

#[derive(Clone, Debug, Deref, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct DocumentProperty(serde_json::Value);

/// Arbitrary properties that can be attached to a document.
pub(crate) type DocumentProperties = HashMap<DocumentPropertyId, DocumentProperty>;

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
    pub(crate) snippet: DocumentSnippet,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,

    /// The tags associated to the document.
    pub(crate) tags: Vec<DocumentTag>,

    /// Embedding from smbert.
    pub(crate) embedding: NormalizedEmbedding,

    /// Indicates if the document is considered for recommendations.
    pub(crate) is_candidate: bool,
}

#[derive(Debug)]
pub(crate) struct ExcerptedDocument {
    pub(crate) id: DocumentId,
    pub(crate) snippet: DocumentSnippet,
    pub(crate) properties: DocumentProperties,
    pub(crate) tags: Vec<DocumentTag>,
    pub(crate) is_candidate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id() {
        assert!(DocumentId::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentId::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentId::try_from("0123456789").is_ok());
        assert!(DocumentId::try_from("_-:@.").is_ok());
        assert!(DocumentId::try_from("").is_err());
        assert!(DocumentId::try_from(["a"; 257].join("")).is_err());
        assert!(DocumentId::try_from("!?ß").is_err());
    }

    #[test]
    fn test_tag() {
        assert!(DocumentTag::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentTag::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentTag::try_from("0123456789").is_ok());
        assert!(DocumentTag::try_from(" .:,;-_#'+*^°!\"§$%&/()=?\\´`@€").is_ok());
        assert!(DocumentTag::try_from("").is_err());
        assert!(DocumentTag::try_from(["a"; 257].join("")).is_err());
        assert!(DocumentTag::try_from("\0").is_err());
    }
}
