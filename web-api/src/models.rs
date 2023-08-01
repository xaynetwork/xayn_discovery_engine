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

use std::{borrow::Borrow, collections::HashMap, ops::RangeInclusive};

use chrono::DateTime;
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

use crate::{
    error::common::{
        InvalidDocumentId,
        InvalidDocumentProperties,
        InvalidDocumentProperty,
        InvalidDocumentPropertyId,
        InvalidDocumentPropertyReason,
        InvalidDocumentQuery,
        InvalidDocumentSnippet,
        InvalidDocumentTag,
        InvalidDocumentTags,
        InvalidString,
        InvalidUserId,
    },
    storage::property_filter::IndexedPropertyType,
};

fn trim(string: &mut String) {
    let trimmed = string.trim();
    if trimmed.len() < string.len() {
        *string = trimmed.to_string();
    }
}

fn validate_string(
    value: &str,
    length_constraints: RangeInclusive<usize>,
    syntax: &'static Regex,
) -> Result<(), InvalidString> {
    let size = value.len();
    if !length_constraints.contains(&size) {
        Err(InvalidString::Size {
            got: size,
            min: *length_constraints.start(),
            max: *length_constraints.end(),
        })
    } else if !syntax.is_match(value) {
        Err(InvalidString::Syntax {
            expected: syntax.as_str(),
        })
    } else {
        Ok(())
    }
}

macro_rules! string_wrapper {
    ($($(#[$attribute:meta])* $visibility:vis $name:ident, $error:ident, $syntax:expr, $full_range:expr);* $(;)?) => {
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

                    let length_constraints = RangeInclusive::from($full_range);
                    validate_string(&value, length_constraints, &*$syntax)
                        .map_err($error)?;

                    Ok(Self(value))
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

static GENERIC_ID_SYNTAX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-:@.][a-zA-Z0-9\-:@._]*$").unwrap());

static PROPERTY_ID_SYNTAX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z0-9\-:@][a-zA-Z0-9\-:@_]*$").unwrap());

static GENERIC_STRING_SYNTAX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\x00]*$").unwrap());

string_wrapper! {
    /// A unique document identifier.
    pub(crate) DocumentId, InvalidDocumentId, GENERIC_ID_SYNTAX, 1..=256;
    /// A unique document property identifier.
    pub(crate) DocumentPropertyId, InvalidDocumentPropertyId, PROPERTY_ID_SYNTAX, 1..=256;
    /// A unique user identifier.
    pub(crate) UserId, InvalidUserId, GENERIC_ID_SYNTAX, 1..=256;
    /// A document tag.
    pub(crate) DocumentTag, InvalidDocumentTag, GENERIC_STRING_SYNTAX, 1..=256;
    /// A document query.
    pub(crate) DocumentQuery, InvalidDocumentQuery, GENERIC_STRING_SYNTAX, 1..=512;
}

/// A document snippet.
#[derive(Clone, Debug, Deref, Deserialize, PartialEq, Serialize, Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct DocumentSnippet(String);

impl DocumentSnippet {
    pub(crate) fn new(
        value: impl Into<String>,
        max_size: usize,
    ) -> Result<Self, InvalidDocumentSnippet> {
        let mut value = value.into();
        trim(&mut value);

        validate_string(&value, 1..=max_size, &GENERIC_STRING_SYNTAX)
            .map_err(InvalidDocumentSnippet)?;

        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Deref, Deserialize, Into, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct DocumentProperty(Value);

impl DocumentProperty {
    pub(crate) fn try_from_value(
        property_id: &DocumentPropertyId,
        mut value: Value,
        max_properties_string_size: usize,
    ) -> Result<Self, InvalidDocumentProperty> {
        let validate_string = |value: &str| {
            validate_string(
                value,
                0..=(max_properties_string_size),
                &GENERIC_STRING_SYNTAX,
            )
        };

        match &mut value {
            Value::Bool(_) | Value::Number(_) | Value::Null => {}
            Value::String(string) => {
                if validate_string(string).is_err() {
                    return Err(InvalidDocumentProperty {
                        property_id: property_id.clone(),
                        invalid_value: value,
                        invalid_reason: InvalidDocumentPropertyReason::InvalidString,
                    });
                }
            }
            Value::Array(array) => {
                if array.len() > 100 {
                    return Err(InvalidDocumentProperty {
                        property_id: property_id.clone(),
                        invalid_value: value.clone(),
                        invalid_reason: InvalidDocumentPropertyReason::InvalidArray,
                    });
                }
                for value in array {
                    let Value::String(ref mut string) = value else {
                        return Err(InvalidDocumentProperty {
                            property_id: property_id.clone(),
                            invalid_value: value.clone(),
                            invalid_reason: InvalidDocumentPropertyReason::IncompatibleType {
                                expected: IndexedPropertyType::Keyword,
                            },
                        });
                    };
                    trim(string);
                    if validate_string(string).is_err() {
                        return Err(InvalidDocumentProperty {
                            property_id: property_id.clone(),
                            invalid_value: value.clone(),
                            invalid_reason: InvalidDocumentPropertyReason::InvalidString,
                        });
                    }
                }
            }
            Value::Object(_) => {
                return Err(InvalidDocumentProperty {
                    property_id: property_id.clone(),
                    invalid_value: value.clone(),
                    invalid_reason: InvalidDocumentPropertyReason::UnsupportedType,
                });
            }
        };

        Ok(Self(value))
    }
}

impl PartialOrd for DocumentProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (&self.0, &other.0) {
            (Value::Number(this), Value::Number(other)) => this
                .as_f64()
                .and_then(|this| other.as_f64().map(|other| this.total_cmp(&other))),
            (Value::String(this), Value::String(other)) => DateTime::parse_from_rfc3339(this)
                .and_then(|this| DateTime::parse_from_rfc3339(other).map(|other| this.cmp(&other)))
                .ok(),
            _ => None,
        }
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
        max_size: usize,
    ) -> Result<Self, InvalidDocumentProperties> {
        if size > max_size {
            return Err(InvalidDocumentProperties::StorageSize {
                got: size,
                max: max_size,
            });
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
        let max = 10;
        if size <= max {
            Ok(Self(tags))
        } else {
            Err(InvalidDocumentTags { size, max })
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

    /// Whether the embedding was computed on the summarized snippet.
    pub(crate) is_summarized: bool,

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
    pub(crate) is_summarized: bool,
    pub(crate) properties: DocumentProperties,
    pub(crate) tags: DocumentTags,
    pub(crate) is_candidate: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    impl TryFrom<Value> for DocumentProperty {
        type Error = InvalidDocumentProperty;

        fn try_from(value: Value) -> Result<Self, Self::Error> {
            DocumentProperty::try_from_value(&"p".try_into().unwrap(), value, 128)
        }
    }

    #[test]
    fn test_is_valid_id() {
        assert!(DocumentId::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentId::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentId::try_from("0123456789").is_ok());
        assert!(DocumentId::try_from("-_:@.").is_ok());

        assert_eq!(
            DocumentId::try_from(""),
            Err(InvalidDocumentId(InvalidString::Size {
                got: 0,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentId::try_from("_"),
            Err(InvalidDocumentId(InvalidString::Syntax {
                expected: GENERIC_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentId::try_from(["a"; 257].join("")),
            Err(InvalidDocumentId(InvalidString::Size {
                got: 257,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentId::try_from("!?ß"),
            Err(InvalidDocumentId(InvalidString::Syntax {
                expected: GENERIC_ID_SYNTAX.as_str()
            }))
        );
    }

    #[test]
    fn test_is_valid_property_id() {
        assert!(DocumentPropertyId::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentPropertyId::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentPropertyId::try_from("0123456789").is_ok());
        assert!(DocumentPropertyId::try_from("-_:@").is_ok());

        assert_eq!(
            DocumentPropertyId::try_from(""),
            Err(InvalidDocumentPropertyId(InvalidString::Size {
                got: 0,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("_"),
            Err(InvalidDocumentPropertyId(InvalidString::Syntax {
                expected: PROPERTY_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("."),
            Err(InvalidDocumentPropertyId(InvalidString::Syntax {
                expected: PROPERTY_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from(["a"; 257].join("")),
            Err(InvalidDocumentPropertyId(InvalidString::Size {
                got: 257,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("!?ß"),
            Err(InvalidDocumentPropertyId(InvalidString::Syntax {
                expected: PROPERTY_ID_SYNTAX.as_str()
            }))
        );
    }

    #[test]
    fn test_is_valid_tag() {
        assert!(DocumentTag::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentTag::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentTag::try_from("0123456789").is_ok());
        assert!(DocumentTag::try_from(" .:,;-_#'+*^°!\"§$%&/()=?\\´`@€").is_ok());

        assert_eq!(
            DocumentTag::try_from(""),
            Err(InvalidDocumentTag(InvalidString::Size {
                got: 0,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentTag::try_from(["a"; 257].join("")),
            Err(InvalidDocumentTag(InvalidString::Size {
                got: 257,
                min: 1,
                max: 256
            }))
        );
        assert_eq!(
            DocumentTag::try_from("\0"),
            Err(InvalidDocumentTag(InvalidString::Syntax {
                expected: GENERIC_STRING_SYNTAX.as_str()
            }))
        );
    }

    #[test]
    fn test_is_valid_query() {
        assert!(DocumentQuery::try_from("abcdefghijklmnopqrstruvwxyz").is_ok());
        assert!(DocumentQuery::try_from("ABCDEFGHIJKLMNOPQURSTUVWXYZ").is_ok());
        assert!(DocumentQuery::try_from("0123456789").is_ok());
        assert!(DocumentQuery::try_from(" .:,;-_#'+*^°!\"§$%&/()=?\\´`@€").is_ok());

        assert_eq!(
            DocumentQuery::try_from(""),
            Err(InvalidDocumentQuery(InvalidString::Size {
                got: 0,
                min: 1,
                max: 512,
            }))
        );
        assert_eq!(
            DocumentQuery::try_from(["a"; 513].join("")),
            Err(InvalidDocumentQuery(InvalidString::Size {
                got: 513,
                min: 1,
                max: 512,
            }))
        );
        assert_eq!(
            DocumentQuery::try_from("\0"),
            Err(InvalidDocumentQuery(InvalidString::Syntax {
                expected: GENERIC_STRING_SYNTAX.as_str()
            }))
        );
    }
}
