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
    fmt::Debug,
    ops::{RangeBounds, RangeInclusive},
    str::FromStr,
};

use chrono::DateTime;
use derive_more::{Deref, DerefMut, Display, Into};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
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
        InvalidEsSnippetIdFormat,
        InvalidString,
        InvalidUserId,
        RangeBoundsInError,
    },
    storage::property_filter::IndexedPropertyType,
    Error,
};

fn trim(string: &mut String) {
    let trimmed = string.trim();
    if trimmed.len() < string.len() {
        *string = trimmed.to_string();
    }
}

fn validate_string(
    value: &str,
    length_constraints: impl RangeBounds<usize>,
    syntax: &'static Regex,
) -> Result<(), InvalidString> {
    let size = value.len();
    if !length_constraints.contains(&size) {
        Err(InvalidString::Size {
            got: size,
            bounds: RangeBoundsInError::new(length_constraints),
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
    ($($(#[$attribute:meta])* $visibility:vis $name:ident, $error:ident, $syntax:expr $(, $full_range:expr)?);* $(;)?) => (
        $(
            string_wrapper!(@base $(#[$attribute])* $visibility $name, $error, $syntax);
            string_wrapper!(@new $visibility $name, $error $(, $full_range)?);
        )*
    );
    (@base $(#[$attribute:meta])* $visibility:vis $name:ident, $error:ident, $syntax:expr) => (
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
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
            Type,
            FromRow,
        )]
        #[serde(transparent)]
        #[sqlx(transparent, no_pg_array)]
        $visibility struct $name(String);

        impl $name {
            $visibility fn new_with_length_constraint(value: impl Into<String>, length_constraints: impl RangeBounds<usize>) -> Result<Self, $error> {
                let mut value = value.into();
                trim(&mut value);
                validate_string(&value, length_constraints, &*$syntax)?;
                Ok(Self(value))
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
    );
    (@new $visibility:vis $name:ident, $error:ident) => ();
    (@new $visibility:vis $name:ident, $error:ident, $full_range:expr) => (
        impl $name {
            $visibility fn new(value: impl Into<String>) -> Result<Self, $error> {
                let length_constraints = RangeInclusive::from($full_range);
                Self::new_with_length_constraint(value.into(), length_constraints)
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
                Self::new(value.to_string())
            }
        }

        impl FromStr for $name {
            type Err = $error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.try_into()
            }
        }
    );
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
    pub(crate) DocumentQuery, InvalidDocumentQuery, GENERIC_STRING_SYNTAX;
    /// A document snippet.
    pub(crate) DocumentSnippet, InvalidDocumentSnippet, GENERIC_STRING_SYNTAX;
}

/// Id pointing to a specific snippet in a document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub(crate) struct SnippetId {
    document_id: DocumentId,
    sub_id: u32,
}

impl SnippetId {
    pub(crate) fn new(document_id: DocumentId, sub_id: u32) -> Self {
        Self {
            document_id,
            sub_id,
        }
    }

    pub(crate) fn document_id(&self) -> &DocumentId {
        &self.document_id
    }

    pub(crate) fn into_document_id(self) -> DocumentId {
        self.document_id
    }

    pub(crate) fn sub_id(&self) -> u32 {
        self.sub_id
    }

    const ES_SNIPPET_ID_PREFIX: &'static str = "_s.";

    pub(crate) fn try_from_es_id(es_id: impl AsRef<str>) -> Result<Self, Error> {
        let es_id = es_id.as_ref();
        if let Some(suffix) = es_id.strip_prefix(Self::ES_SNIPPET_ID_PREFIX) {
            let Some((sub_id, document_id)) = suffix.split_once('.') else {
                return Err(InvalidEsSnippetIdFormat { id: es_id.into() }.into());
            };
            let Ok(sub_id) = sub_id.parse() else {
                return Err(InvalidEsSnippetIdFormat { id: es_id.into() }.into());
            };
            let Ok(document_id) = document_id.parse() else {
                return Err(InvalidEsSnippetIdFormat { id: es_id.into() }.into());
            };
            Ok(Self::new(document_id, sub_id))
        } else {
            let Ok(document_id) = es_id.parse() else {
                return Err(InvalidEsSnippetIdFormat { id: es_id.into() }.into());
            };
            Ok(Self::new(document_id, 0))
        }
    }

    pub(crate) fn to_es_id(&self) -> Cow<'_, str> {
        if self.sub_id == 0 {
            Cow::Borrowed(&self.document_id)
        } else {
            Cow::Owned(format!(
                "{}{}.{}",
                Self::ES_SNIPPET_ID_PREFIX,
                self.sub_id,
                self.document_id,
            ))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SnippetOrDocumentId {
    SnippetId(SnippetId),
    DocumentId(DocumentId),
}

impl SnippetOrDocumentId {
    pub(crate) fn document_id(&self) -> &DocumentId {
        match self {
            SnippetOrDocumentId::SnippetId(id) => id.document_id(),
            SnippetOrDocumentId::DocumentId(id) => id,
        }
    }

    pub(crate) fn sub_id(&self) -> Option<u32> {
        match self {
            SnippetOrDocumentId::SnippetId(id) => Some(id.sub_id()),
            SnippetOrDocumentId::DocumentId(_) => None,
        }
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
                0..=max_properties_string_size,
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
#[sqlx(transparent, no_pg_array)]
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

#[derive(Clone, Debug)]
pub(crate) struct SnippetForInteraction {
    pub(crate) id: SnippetId,
    pub(crate) embedding: NormalizedEmbedding,
    pub(crate) tags: DocumentTags,
}

/// Represents a result from a personalization query.
#[derive(Clone, Debug)]
pub(crate) struct PersonalizedDocument {
    /// Unique identifier of the document.
    pub(crate) id: SnippetId,

    /// Similarity score of the personalized document.
    pub(crate) score: f32,

    /// Embedding from smbert.
    pub(crate) embedding: NormalizedEmbedding,

    /// User-defined document properties.
    ///
    /// Depending on the context the properties might not be loaded from the db.
    pub(crate) properties: Option<DocumentProperties>,

    /// Snippet of the document.
    ///
    /// Depending on the context the snippet might not be loaded from the db.
    pub(crate) snippet: Option<DocumentSnippet>,

    /// The tags associated to the document.
    pub(crate) tags: DocumentTags,

    /// Additional data about the document that can be helpful while tuning or debugging the system.
    pub(crate) dev: Option<DocumentDevData>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct DocumentDevData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) raw_scores: Option<RawScores>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct RawScores {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) knn: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bm25: Option<f32>,
}

impl AiDocument for PersonalizedDocument {
    type Id = SnippetId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn embedding(&self) -> &NormalizedEmbedding {
        &self.embedding
    }
}

/// Represents a document sent for ingestion.
#[derive(Clone, Debug)]
pub(crate) struct DocumentForIngestion {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// The sha256 hash of the original document provided by the client.
    pub(crate) original_sha256: Sha256Hash,

    /// Snippet used to calculate embeddings for a document.
    pub(crate) snippets: Vec<DocumentContent>,

    /// Method used to preprocess the document before ingestion.
    pub(crate) preprocessing_step: PreprocessingStep,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,

    /// The tags associated to the document.
    pub(crate) tags: DocumentTags,

    /// Indicates if the document is considered for recommendations.
    pub(crate) is_candidate: bool,
}

#[derive(Clone, Debug, PartialEq, Type)]
#[sqlx(transparent)]
pub(crate) struct Sha256Hash([u8; 32]);

impl Sha256Hash {
    pub(crate) fn zero() -> Self {
        Self([0; 32])
    }

    pub(crate) fn calculate(document: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(document);
        Self(hasher.finalize().into())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DocumentContent {
    pub(crate) snippet: DocumentSnippet,
    pub(crate) embedding: NormalizedEmbedding,
}

#[derive(Debug)]
pub(crate) struct ExcerptedDocument {
    pub(crate) id: DocumentId,
    pub(crate) original_sha256: Sha256Hash,
    pub(crate) preprocessing_step: PreprocessingStep,
    pub(crate) properties: DocumentProperties,
    pub(crate) tags: DocumentTags,
    pub(crate) is_candidate: bool,
}

/// The preprocessing step used on the raw document.
// Note: The same input parameter (e.g. split) can over time
//       map to different variants, e.g. now it maps to `CuttersSplit`
//       but in the future it will map to different splits.
//       This matters for deciding if in case of reingestion we should
//       reprocess the document even if the raw document didn't change.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "preprocessing_step", rename_all = "snake_case")]
pub(crate) enum PreprocessingStep {
    None,
    Summarize,
    CuttersSplit,
    NltkSplitV1,
}

impl PreprocessingStep {
    pub(crate) fn default_split() -> Self {
        Self::NltkSplitV1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personalization::SemanticSearchConfig;

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
            Err(InvalidDocumentId::from(InvalidString::Size {
                got: 0,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentId::try_from("_"),
            Err(InvalidDocumentId::from(InvalidString::Syntax {
                expected: GENERIC_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentId::try_from(["a"; 257].join("")),
            Err(InvalidDocumentId::from(InvalidString::Size {
                got: 257,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentId::try_from("!?ß"),
            Err(InvalidDocumentId::from(InvalidString::Syntax {
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
            Err(InvalidDocumentPropertyId::from(InvalidString::Size {
                got: 0,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("_"),
            Err(InvalidDocumentPropertyId::from(InvalidString::Syntax {
                expected: PROPERTY_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("."),
            Err(InvalidDocumentPropertyId::from(InvalidString::Syntax {
                expected: PROPERTY_ID_SYNTAX.as_str()
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from(["a"; 257].join("")),
            Err(InvalidDocumentPropertyId::from(InvalidString::Size {
                got: 257,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentPropertyId::try_from("!?ß"),
            Err(InvalidDocumentPropertyId::from(InvalidString::Syntax {
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
            Err(InvalidDocumentTag::from(InvalidString::Size {
                got: 0,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentTag::try_from(["a"; 257].join("")),
            Err(InvalidDocumentTag::from(InvalidString::Size {
                got: 257,
                bounds: (1..=256).into(),
            }))
        );
        assert_eq!(
            DocumentTag::try_from("\0"),
            Err(InvalidDocumentTag::from(InvalidString::Syntax {
                expected: GENERIC_STRING_SYNTAX.as_str()
            }))
        );
    }

    #[test]
    fn test_is_valid_query() {
        let config = SemanticSearchConfig::default();
        let bounds = config.query_size_bounds();

        assert!(DocumentQuery::new_with_length_constraint(
            "abcdefghijklmnopqrstruvwxyz",
            bounds.clone()
        )
        .is_ok());
        assert!(DocumentQuery::new_with_length_constraint(
            "ABCDEFGHIJKLMNOPQURSTUVWXYZ",
            bounds.clone()
        )
        .is_ok());
        assert!(DocumentQuery::new_with_length_constraint("0123456789", bounds.clone()).is_ok());
        assert!(DocumentQuery::new_with_length_constraint(
            " .:,;-_#'+*^°!\"§$%&/()=?\\´`@€",
            bounds.clone()
        )
        .is_ok());

        assert_eq!(
            DocumentQuery::new_with_length_constraint("", bounds.clone()),
            Err(InvalidDocumentQuery::from(InvalidString::Size {
                got: 0,
                bounds: (1..=512).into(),
            }))
        );
        assert_eq!(
            DocumentQuery::new_with_length_constraint(["a"; 513].join(""), bounds.clone()),
            Err(InvalidDocumentQuery::from(InvalidString::Size {
                got: 513,
                bounds: (1..=512).into(),
            }))
        );
        assert_eq!(
            DocumentQuery::new_with_length_constraint("\0", bounds),
            Err(InvalidDocumentQuery::from(InvalidString::Syntax {
                expected: GENERIC_STRING_SYNTAX.as_str()
            }))
        );
    }
}
