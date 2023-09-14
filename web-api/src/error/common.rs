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
    borrow::Cow,
    fmt::{Debug, Display},
    ops::{Bound, RangeBounds},
};

use actix_web::http::StatusCode;
use derive_more::From;
use displaydoc::Display;
use serde::{Serialize, Serializer};
use serde_json::Value;
use thiserror::Error;
use tracing::Level;
use xayn_ai_bert::InvalidEmbedding;
use xayn_snippet_extractor::pool::PoolAcquisitionError;
use xayn_web_api_shared::elastic;

use super::application::{impl_application_error, ApplicationError};
use crate::{
    models::{DocumentId, DocumentPropertyId},
    storage::property_filter::{IncompatibleUpdate, IndexedPropertyType},
    Error,
};

impl_application_error!(InvalidEmbedding => INTERNAL_SERVER_ERROR, ERROR);

/// The requested document was not found.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct DocumentNotFound;

impl_application_error!(DocumentNotFound => BAD_REQUEST, INFO);

/// The requested document was found but not the requested property.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct DocumentPropertyNotFound;

impl_application_error!(DocumentPropertyNotFound => BAD_REQUEST, INFO);

#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidString {
    /// Invalid byte size. Got {got}, expected {bounds:?}.
    Size {
        got: usize,
        bounds: RangeBoundsInError,
    },
    /// Invalid syntax, expected: {expected}
    Syntax { expected: &'static str },
}

#[cfg_attr(test, derive(PartialEq))]
pub(crate) struct RangeBoundsInError {
    start: Bound<usize>,
    end: Bound<usize>,
}

impl RangeBoundsInError {
    pub(crate) fn new(bound: impl RangeBounds<usize>) -> Self {
        Self {
            start: bound.start_bound().cloned(),
            end: bound.end_bound().cloned(),
        }
    }
}

impl<T> From<T> for RangeBoundsInError
where
    T: RangeBounds<usize>,
{
    fn from(value: T) -> Self {
        RangeBoundsInError::new(value)
    }
}

impl Debug for RangeBoundsInError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.start {
            Bound::Included(bound) => write!(f, "{bound}")?,
            Bound::Excluded(bound) => write!(f, "{bound}<")?,
            Bound::Unbounded => (),
        }

        f.write_str("..")?;

        match self.end {
            Bound::Included(bound) => write!(f, "={bound}")?,
            Bound::Excluded(bound) => write!(f, "{bound}")?,
            Bound::Unbounded => (),
        }

        Ok(())
    }
}

impl Serialize for RangeBoundsInError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{self:?}").serialize(serializer)
    }
}

/// Malformed user id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[serde(transparent)]
pub(crate) struct InvalidUserId(#[from] InvalidString);

impl_application_error!(InvalidUserId => BAD_REQUEST, INFO);

/// Malformed document id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentId(#[from] InvalidString);

impl_application_error!(InvalidDocumentId => BAD_REQUEST, INFO);

/// Malformed document property id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentPropertyId(#[from] InvalidString);

impl_application_error!(InvalidDocumentPropertyId => BAD_REQUEST, INFO);

/// Invalid ES snippet id: {id}
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct InvalidEsSnippetIdFormat {
    pub(crate) id: String,
}

impl_application_error!(InvalidEsSnippetIdFormat => INTERNAL_SERVER_ERROR, ERROR);

/// Malformed document property {property_id}, {invalid_reason}: {invalid_value}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
// there are some false positives with clippy and displaydoc
#[allow(clippy::doc_markdown)]
pub(crate) struct InvalidDocumentProperty {
    pub(crate) property_id: DocumentPropertyId,
    pub(crate) invalid_value: Value,
    pub(crate) invalid_reason: InvalidDocumentPropertyReason,
}

#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum InvalidDocumentPropertyReason {
    /// unsupported type
    UnsupportedType,
    /// malformed datetime string
    MalformedDateTimeString,
    /// incompatible type (expected {expected:?})
    IncompatibleType { expected: IndexedPropertyType },
    /// string too long or contains invalid characters
    InvalidString,
    /// array too long
    InvalidArray,
    /// unindexed id
    UnindexedId,
}

impl_application_error!(InvalidDocumentProperty => BAD_REQUEST, INFO);

#[derive(Debug, Error, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidDocumentProperties {
    /// Storage byte size of document properties is to large. Got {got}, expected at most {max}.
    StorageSize { got: usize, max: usize },
}

impl_application_error!(InvalidDocumentProperties => BAD_REQUEST, INFO);

/// Malformed document tag: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentTag(#[from] InvalidString);

impl_application_error!(InvalidDocumentTag => BAD_REQUEST, INFO);

/// To many document tags. Got {size}, expect at most {max}.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct InvalidDocumentTags {
    pub(crate) size: usize,
    pub(crate) max: usize,
}

impl_application_error!(InvalidDocumentTags => BAD_REQUEST, INFO);

#[derive(Debug, Error, Display, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidDocumentSnippet {
    /// Malformed document snippet: {0}
    InvalidString(#[from] InvalidString),
    /// Input document didn't yield any snippets
    NoSnippets {},
    /// File is not base64 encoded
    FileNotBase64Encoded,
}

impl_application_error!(InvalidDocumentSnippet => BAD_REQUEST, INFO);

/// Binary upload feature it is not available.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct FileUploadNotEnabled;

impl_application_error!(FileUploadNotEnabled => BAD_REQUEST, INFO);

/// Content-type of the uploaded file is not supported.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) enum InvalidBinary {
    /// Unrecognized content-type.
    Unrecognized,
    /// Unsupported media type. Found {found}
    MediaType { found: String },
    /// Invalid content
    InvalidContent,
}

impl_application_error!(InvalidBinary => BAD_REQUEST, INFO);

/// Malformed document query: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentQuery(#[from] InvalidString);

impl_application_error!(InvalidDocumentQuery => BAD_REQUEST, INFO);

/// Malsized document count. Got {count}, expected {min}..={max}.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct InvalidDocumentCount {
    pub(crate) count: usize,
    pub(crate) min: usize,
    pub(crate) max: usize,
}

impl_application_error!(InvalidDocumentCount => BAD_REQUEST, INFO);

#[derive(Debug, Display, Error, Serialize)]
pub(crate) enum ForbiddenDevOption {
    /// Dev options are not enabled for this tentant
    DevDisabled,
}

impl_application_error!(ForbiddenDevOption => FORBIDDEN, INFO);

/// Failed to delete some documents.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct FailedToDeleteSomeDocuments {
    pub(crate) errors: Vec<DocumentIdAsObject>,
}

impl_application_error!(FailedToDeleteSomeDocuments => BAD_REQUEST, INFO);

/// The validation of some documents failed.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct FailedToValidateDocuments {
    pub(crate) documents: Vec<DocumentInBatchError>,
}

impl_application_error!(FailedToValidateDocuments => BAD_REQUEST, INFO);

#[derive(Serialize, Debug)]
pub(crate) struct DocumentInBatchError {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) details: Value,
}

impl DocumentInBatchError {
    pub(crate) fn new(id: impl Into<String>, error: &dyn ApplicationError) -> Self {
        Self {
            id: id.into(),
            kind: error.kind().into(),
            details: error.encode_details(),
        }
    }
}
/// The ingestion of some documents failed.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct FailedToIngestDocuments {
    pub(crate) documents: Vec<DocumentInBatchError>,
}

impl_application_error!(FailedToIngestDocuments => INTERNAL_SERVER_ERROR, ERROR);

/// Failed to set some document candidates.
#[derive(Debug, Display, Error, Serialize)]
pub(crate) struct FailedToSetSomeDocumentCandidates {
    pub(crate) documents: Vec<DocumentIdAsObject>,
}

impl_application_error!(FailedToSetSomeDocumentCandidates => BAD_REQUEST, INFO);

/// The history does not contains enough information.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct HistoryTooSmall;

impl_application_error!(HistoryTooSmall => BAD_REQUEST, INFO);

impl_application_error!(IncompatibleUpdate => BAD_REQUEST, INFO);

/// Custom error for 400 Bad Request status code: {message}
#[derive(Debug, Error, Display, Serialize, From)]
pub(crate) struct BadRequest {
    pub(crate) message: Cow<'static, str>,
}

impl_application_error!(BadRequest => BAD_REQUEST, INFO);

impl From<&'static str> for BadRequest {
    fn from(message: &'static str) -> Self {
        Self {
            message: Cow::Borrowed(message),
        }
    }
}

impl From<String> for BadRequest {
    fn from(message: String) -> Self {
        Self {
            message: Cow::Owned(message),
        }
    }
}

impl From<elastic::Error> for Error {
    fn from(error: elastic::Error) -> Self {
        InternalError::from_std(error).into()
    }
}

impl From<chrono::ParseError> for Error {
    fn from(error: chrono::ParseError) -> Self {
        InternalError::from_std(error).into()
    }
}

#[derive(Serialize, Debug, From)]
#[from(types(DocumentId))]
pub(crate) struct DocumentIdAsObject {
    pub(crate) id: String,
}

/// Internal Error: {0}
#[derive(Debug, Display, Error)]
pub(crate) struct InternalError(anyhow::Error);

impl InternalError {
    pub(crate) fn from_message(msg: impl Display + Debug + Send + Sync + 'static) -> Self {
        Self::from_anyhow(anyhow::Error::msg(msg))
    }

    pub(crate) fn from_std(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(anyhow::Error::new(error))
    }

    pub(crate) fn from_anyhow(error: anyhow::Error) -> Self {
        Self(error)
    }
}

impl ApplicationError for InternalError {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn kind(&self) -> &str {
        "InternalServerError"
    }

    fn level(&self) -> Level {
        Level::ERROR
    }

    fn encode_details(&self) -> Value {
        Value::Null
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        InternalError::from_anyhow(error).into()
    }
}

macro_rules! impl_from_std_error {
    ($($error:ty,)*) => {$(
        impl From<$error> for Error {
            fn from(error: $error) -> Self {
                InternalError::from_std(error).into()
            }
        }
    )*};
}

impl_from_std_error!(
    sqlx::Error,
    reqwest::Error,
    std::io::Error,
    tokio::task::JoinError,
    serde_json::Error,
    xayn_snippet_extractor::Error,
);

impl ApplicationError for PoolAcquisitionError {
    fn status_code(&self) -> StatusCode {
        if self.is_timeout() {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }

    fn kind(&self) -> &str {
        if self.is_timeout() {
            "ServiceOverloaded"
        } else {
            "InternalServerError"
        }
    }

    fn level(&self) -> Level {
        if self.is_timeout() {
            Level::WARN
        } else {
            Level::ERROR
        }
    }

    fn encode_details(&self) -> Value {
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_bound_formatting() {
        assert_eq!(format!("{:?}", RangeBoundsInError::new(..)), "..");
        assert_eq!(format!("{:?}", RangeBoundsInError::new(1..)), "1..");
        assert_eq!(format!("{:?}", RangeBoundsInError::new(..1)), "..1");
        assert_eq!(format!("{:?}", RangeBoundsInError::new(1..2)), "1..2");
        assert_eq!(format!("{:?}", RangeBoundsInError::new(..=1)), "..=1");
        assert_eq!(format!("{:?}", RangeBoundsInError::new(1..=2)), "1..=2");
    }
}
