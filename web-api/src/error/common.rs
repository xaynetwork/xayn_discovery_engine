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

use std::borrow::Cow;

use actix_web::http::StatusCode;
use derive_more::From;
use displaydoc::Display;
use serde::Serialize;
use thiserror::Error;

use super::application::ApplicationError;
use crate::{impl_application_error, models::DocumentId, Error};

/// The requested document was not found.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct DocumentNotFound;

impl_application_error!(DocumentNotFound => NOT_FOUND);

/// The requested document was found but not the requested property.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct DocumentPropertyNotFound;

impl_application_error!(DocumentPropertyNotFound => NOT_FOUND);

/// The requested property was not found.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct PropertyNotFound;

impl_application_error!(PropertyNotFound => NOT_FOUND);

/// Malformed user id.
#[derive(Debug, Error, Display, Serialize)]
pub struct InvalidUserId {
    pub(crate) id: String,
}

impl_application_error!(InvalidUserId => BAD_REQUEST);

/// Malformed document id.
#[derive(Debug, Error, Display, Serialize)]
pub struct InvalidDocumentId {
    pub(crate) id: String,
}

impl_application_error!(InvalidDocumentId => BAD_REQUEST);

/// Malformed document property id.
#[derive(Debug, Error, Display, Serialize)]
pub struct InvalidDocumentPropertyId {
    pub(crate) id: String,
}

impl_application_error!(InvalidDocumentPropertyId => BAD_REQUEST);

/// Not enough interactions.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct NotEnoughInteractions;

impl_application_error!(NotEnoughInteractions => CONFLICT);

/// Failed to delete some documents
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct FailedToDeleteSomeDocuments {
    pub(crate) errors: Vec<DocumentIdAsObject>,
}

impl_application_error!(FailedToDeleteSomeDocuments => INTERNAL_SERVER_ERROR);

/// The ingestion of some documents failed.
#[derive(Debug, Error, Display, Serialize)]
pub(crate) struct IngestingDocumentsFailed {
    pub(crate) documents: Vec<DocumentIdAsObject>,
}

impl_application_error!(IngestingDocumentsFailed => INTERNAL_SERVER_ERROR);

/// Custom error for 400 Bad Request status code.
#[derive(Debug, Error, Display, Serialize, From)]
pub(crate) struct BadRequest {
    pub(crate) message: Cow<'static, str>,
}

impl_application_error!(BadRequest => BAD_REQUEST);

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

#[derive(Serialize, Debug, From)]
pub(crate) struct DocumentIdAsObject {
    pub(crate) id: DocumentId,
}

/// Internal Error: {0}
#[derive(Debug, Display, Error)]
pub(crate) struct InternalError(anyhow::Error);

impl InternalError {
    pub(crate) fn from_message(msg: &'static str) -> Self {
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

    fn encode_details(&self) -> serde_json::Value {
        serde_json::Value::Null
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
