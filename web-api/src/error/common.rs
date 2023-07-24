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
};

use actix_web::http::StatusCode;
use derive_more::From;
use displaydoc::Display;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use tracing::Level;
use xayn_ai_bert::InvalidEmbedding;
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

#[derive(Debug, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidString {
    /// Invalid size. Got {got}, expected {min}..={max}.
    Size { got: usize, min: usize, max: usize },
    /// Invalid syntax, expected: {expected}
    Syntax { expected: &'static str },
}

/// Malformed user id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[serde(transparent)]
pub(crate) struct InvalidUserId(pub(crate) InvalidString);

impl_application_error!(InvalidUserId => BAD_REQUEST, INFO);

/// Malformed document id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentId(pub(crate) InvalidString);

impl_application_error!(InvalidDocumentId => BAD_REQUEST, INFO);

/// Malformed document property id: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentPropertyId(pub(crate) InvalidString);

impl_application_error!(InvalidDocumentPropertyId => BAD_REQUEST, INFO);

/// Malformed property {property}, {invalid_reason}: {invalid_value}
#[derive(Debug, Error, Display, Serialize)]
// there are some false positives with clippy and displaydoc
#[allow(clippy::doc_markdown)]
pub(crate) struct InvalidDocumentProperty {
    pub(crate) property: DocumentPropertyId,
    pub(crate) invalid_value: Value,
    pub(crate) invalid_reason: InvalidDocumentPropertyReason,
}

#[derive(Debug, Error, Display, Serialize)]
pub(crate) enum InvalidDocumentPropertyReason {
    /// unsupported value
    UnsupportedType,
    /// malformed datetime string
    MalformedDateTimeString,
    /// incompatible type (expected {expected:?})
    IncompatibleType { expected: IndexedPropertyType },
    /// string too long or contains invalid characters
    InvalidString,
    /// array too long
    InvalidArray,
}

impl_application_error!(InvalidDocumentProperty => BAD_REQUEST, INFO);

/// Storage size of properties is to large. Got {size}, expected {max_size}.
#[derive(Debug, Error, Display, Serialize)]
#[allow(clippy::doc_markdown)]
pub(crate) struct InvalidDocumentProperties {
    pub(crate) size: usize,
    pub(crate) max_size: usize,
}

impl_application_error!(InvalidDocumentProperties => BAD_REQUEST, INFO);

/// Malformed document tag: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentTag(pub(crate) InvalidString);

impl_application_error!(InvalidDocumentTag => BAD_REQUEST, INFO);

/// To many document tags. Got {size}, expect at most {max}.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct InvalidDocumentTags {
    pub(crate) size: usize,
    pub(crate) max: usize,
}

impl_application_error!(InvalidDocumentTags => BAD_REQUEST, INFO);

/// Malformed document snippet: {0}
#[derive(Debug, Error, Display, Serialize)]
#[serde(transparent)]
pub(crate) struct InvalidDocumentSnippet(pub(crate) InvalidString);

impl_application_error!(InvalidDocumentSnippet => BAD_REQUEST, INFO);

/// Malformed document query: {0}
#[derive(Debug, Error, Display, Serialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(transparent)]
pub(crate) struct InvalidDocumentQuery(pub(crate) InvalidString);

impl_application_error!(InvalidDocumentQuery => BAD_REQUEST, INFO);

/// Malsized document count. Got {count}, expected {min}..={max}.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct InvalidDocumentCount {
    pub(crate) count: usize,
    pub(crate) min: usize,
    pub(crate) max: usize,
}

impl_application_error!(InvalidDocumentCount => BAD_REQUEST, INFO);

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

#[derive(Serialize, Debug, From)]
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
);
