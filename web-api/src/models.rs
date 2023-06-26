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

use derive_more::{Deref, DerefMut, Display, Into};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{
    postgres::{PgHasArrayType, PgTypeInfo},
    FromRow,
    Type,
};
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::Document as AiDocument;

use crate::error::common::{
    InvalidDocumentId,
    InvalidDocumentProperties,
    InvalidDocumentProperty,
    InvalidDocumentPropertyId,
    InvalidDocumentSnippet,
    InvalidDocumentTag,
    InvalidDocumentTags,
    InvalidUserId,
};

fn trim(string: &mut String) {
    let trimmed = string.trim();
    if trimmed.len() < string.len() {
        *string = trimmed.to_string();
    }
}

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
                    trim(&mut value);

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
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-:@.][a-zA-Z0-9\-:@._]*$").unwrap());

    (1..=256).contains(&value.len()) && RE.is_match(value)
}

fn is_valid_property_id(value: &str) -> bool {
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-:@][a-zA-Z0-9\-:@_]*$").unwrap());

    (1..=256).contains(&value.len()) && RE.is_match(value)
}

fn is_valid_string(value: &str, len: usize) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\x00]+$").unwrap());

    (1..=len).contains(&value.len()) && RE.is_match(value)
}

fn is_valid_string_empty_ok(value: &str, len: usize) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\x00]*$").unwrap());

    (0..=len).contains(&value.len()) && RE.is_match(value)
}

string_wrapper! {
    /// A unique document identifier.
    pub(crate) DocumentId, InvalidDocumentId, is_valid_id;
    /// A unique document property identifier.
    pub(crate) DocumentPropertyId, InvalidDocumentPropertyId, is_valid_property_id;
    /// A unique user identifier.
    pub(crate) UserId, InvalidUserId, is_valid_id;
    /// A document snippet.
    pub(crate) DocumentSnippet, InvalidDocumentSnippet, |value| is_valid_string(value, 2_048);
    /// A document tag.
    pub(crate) DocumentTag, InvalidDocumentTag, |value| is_valid_string(value, 256);
}

#[derive(Clone, Debug, Deref, Deserialize, Into, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct DocumentProperty(Value);

impl TryFrom<Value> for DocumentProperty {
    type Error = InvalidDocumentProperty;

    fn try_from(mut property: Value) -> Result<Self, Self::Error> {
        match &mut property {
            Value::Bool(_) | Value::Number(_) | Value::Null => {}
            Value::String(string) => {
                if !is_valid_string_empty_ok(string, 2_048) {
                    return Err(InvalidDocumentProperty { value: property });
                }
            }
            Value::Array(array) => {
                for value in array {
                    let Value::String(ref mut string) = value else {
                        return Err(InvalidDocumentProperty { value: property });
                    };
                    trim(string);
                    if !is_valid_string(string, 2_048) {
                        return Err(InvalidDocumentProperty { value: property });
                    }
                }
            }
            Value::Object(_) => return Err(InvalidDocumentProperty { value: property }),
        };

        Ok(Self(property))
    }
}

/// Arbitrary properties that can be attached to a document.
#[derive(Clone, Debug, Default, Deref, DerefMut, Deserialize, FromRow, PartialEq, Serialize)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct DocumentProperties(HashMap<DocumentPropertyId, DocumentProperty>);

impl DocumentProperties {
    pub(crate) fn new(
        properties: HashMap<DocumentPropertyId, DocumentProperty>,
        size: usize,
    ) -> Result<Self, InvalidDocumentProperties> {
        if size > 2_560 {
            return Err(InvalidDocumentProperties { size });
        }

        Ok(Self(properties))
    }
}

impl IntoIterator for DocumentProperties {
    type Item = <HashMap<DocumentPropertyId, DocumentProperty> as IntoIterator>::Item;
    type IntoIter = <HashMap<DocumentPropertyId, DocumentProperty> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Arbitrary tags that can be associated to a document.
#[derive(Clone, Debug, Default, Deref, Deserialize, PartialEq, Serialize, Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct DocumentTags(Vec<DocumentTag>);

impl TryFrom<Vec<DocumentTag>> for DocumentTags {
    type Error = InvalidDocumentTags;

    fn try_from(tags: Vec<DocumentTag>) -> Result<Self, Self::Error> {
        let size = tags.len();
        if size <= 10 {
            Ok(Self(tags))
        } else {
            Err(InvalidDocumentTags { size })
        }
    }
}

impl<'a> IntoIterator for &'a DocumentTags {
    type Item = <&'a Vec<DocumentTag> as IntoIterator>::Item;
    type IntoIter = <&'a Vec<DocumentTag> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
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
    pub(crate) tags: DocumentTags,
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
    pub(crate) tags: DocumentTags,
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
    pub(crate) tags: DocumentTags,

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
    pub(crate) tags: DocumentTags,
    pub(crate) is_candidate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_id() {
        assert!(is_valid_id("abcdefghijklmnopqrstruvwxyz"));
        assert!(is_valid_id("ABCDEFGHIJKLMNOPQURSTUVWXYZ"));
        assert!(is_valid_id("0123456789"));
        assert!(is_valid_id("-_:@."));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("_"));
        assert!(!is_valid_id(&["a"; 257].join("")));
        assert!(!is_valid_id("!?ß"));
    }

    #[test]
    fn test_is_valid_property_id() {
        assert!(is_valid_property_id("abcdefghijklmnopqrstruvwxyz"));
        assert!(is_valid_property_id("ABCDEFGHIJKLMNOPQURSTUVWXYZ"));
        assert!(is_valid_property_id("0123456789"));
        assert!(is_valid_property_id("-_:@"));
        assert!(!is_valid_property_id(""));
        assert!(!is_valid_property_id("_"));
        assert!(!is_valid_property_id("."));
        assert!(!is_valid_property_id(&["a"; 257].join("")));
        assert!(!is_valid_property_id("!?ß"));
    }

    #[test]
    fn test_is_valid_string() {
        assert!(is_valid_string("abcdefghijklmnopqrstruvwxyz", 256));
        assert!(is_valid_string("ABCDEFGHIJKLMNOPQURSTUVWXYZ", 256));
        assert!(is_valid_string("0123456789", 256));
        assert!(is_valid_string(" .:,;-_#'+*^°!\"§$%&/()=?\\´`@€", 256));
        assert!(!is_valid_string("", 256));
        assert!(!is_valid_string(&["a"; 257].join(""), 256));
        assert!(!is_valid_string("\0", 256));
    }

    #[test]
    fn test_is_valid_string_empty_ok() {
        assert!(is_valid_string_empty_ok("abcdefghijklmnopqrstruvwxyz", 256));
        assert!(is_valid_string_empty_ok("ABCDEFGHIJKLMNOPQURSTUVWXYZ", 256));
        assert!(is_valid_string_empty_ok("0123456789", 256));
        assert!(is_valid_string_empty_ok(
            " .:,;-_#'+*^°!\"§$%&/()=?\\´`@€",
            256
        ));
        assert!(is_valid_string_empty_ok("", 256));
        assert!(!is_valid_string_empty_ok(&["a"; 257].join(""), 256));
        assert!(!is_valid_string_empty_ok("\0", 256));
    }
}
