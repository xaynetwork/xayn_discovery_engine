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

use actix_web::http::StatusCode;
use displaydoc::Display;
use serde::Serialize;
use thiserror::Error;

use crate::{impl_application_error, models::DocumentId, Error};

use super::application::ApplicationError;

/// The requested document was not found.
#[derive(Debug, Error, Display, Serialize)]
pub struct DocumentNotFound;
impl_application_error!(DocumentNotFound => NOT_FOUND);

/// The requested property was not found.
#[derive(Debug, Error, Display, Serialize)]
pub struct PropertyNotFound;
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
pub struct NotEnoughInteractions;
impl_application_error!(NotEnoughInteractions => NOT_FOUND);

/// The ingestion of some documents failed.
#[derive(Debug, Error, Display, Serialize)]
pub struct IngestingDocumentsFailed {
    documents: Vec<MappedDocumentId>,
}

#[derive(Serialize, Debug)]
struct MappedDocumentId {
    id: DocumentId,
}

impl_application_error!(IngestingDocumentsFailed => INTERNAL_SERVER_ERROR);

/// Internal Error: {0}
#[derive(Debug, Display, Error)]
pub struct InternalError(anyhow::Error);

impl InternalError {
    #[allow(dead_code)]
    pub fn from_std(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(anyhow::Error::new(error))
    }

    #[allow(dead_code)]
    pub fn from_anyhow(error: anyhow::Error) -> Self {
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

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        InternalError::from_std(error).into()
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        InternalError::from_std(error).into()
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        InternalError::from_anyhow(error).into()
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        InternalError::from_std(error).into()
    }
}

impl From<tokio::task::JoinError> for Error {
    fn from(error: tokio::task::JoinError) -> Self {
        InternalError::from_std(error).into()
    }
}
