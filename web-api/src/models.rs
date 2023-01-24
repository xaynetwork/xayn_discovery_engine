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

use std::{collections::HashMap, str::FromStr};

use derive_more::{AsRef, Display, Into};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use xayn_ai_bert::NormalizedEmbedding;
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
                AsRef,
                Into,
                Clone,
                Debug,
                Display,
                PartialEq,
                Eq,
                Hash,
                Serialize,
                Deserialize,
                sqlx::Type,
                sqlx::FromRow,
            )]
            #[sqlx(transparent)]
            #[serde(try_from = "String", into = "String")]
            $visibility struct $name(String);

            impl $name {
                $visibility fn new(value: impl Into<String> + AsRef<str>) -> Result<Self, $error> {
                    if $is_valid(value.as_ref()) {
                        Ok(Self(value.into()))
                    } else {
                        Err($error { value: value.into() })
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

            impl FromStr for $name {
                type Err = $error;

                fn from_str(value: &str) -> Result<Self, Self::Err> {
                    Self::new(value)
                }
            }
        )*
    };
}

static ID_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_\-:@.]{1,256}$").unwrap());
static TAG_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9.:,;\-_+*!?%&/\\() ]{1,100}$").unwrap());

id_wrapper! {
    pub(crate) DocumentId, InvalidDocumentId, |id| ID_REGEX.is_match(id);
    pub(crate) DocumentPropertyId, InvalidDocumentPropertyId, |id| ID_REGEX.is_match(id);
    pub(crate) UserId, InvalidUserId, |id| ID_REGEX.is_match(id);
    pub(crate) DocumentTag, InvalidDocumentTag, |tag| TAG_REGEX.is_match(tag);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

    fn bert_embedding(&self) -> &NormalizedEmbedding {
        &self.embedding
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) enum UserInteractionType {
    Positive = 1,
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
        assert!(DocumentTag::new(".:,;-_+*!?%&/\\() ").is_ok());
        assert!(DocumentTag::new("").is_err());
        assert!(DocumentTag::new(["a"; 101].join("")).is_err());
        assert!(DocumentTag::new("ß@|").is_err());
    }
}
