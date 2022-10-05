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

use std::{collections::HashMap, ops::RangeInclusive, string::FromUtf8Error};

use derive_more::{AsRef, Display};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warp::{http::StatusCode, reject::Reject, reply, Reply};

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};

/// The range of the count parameter.
pub(crate) const COUNT_PARAM_RANGE: RangeInclusive<usize> = 1..=100;

/// Web API errors.
#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum Error {
    /// [`UserId`] can't be empty.
    UserIdEmpty,

    /// [`UserId`] can't contain NUL character.
    UserIdContainsNul,

    /// Failed to decode [`UserId] from path param: {0}.
    UserIdUtf8Conversion(#[from] FromUtf8Error),

    /// Invalid value for count parameter: {0}. It must be in [`COUNT_PARAM_RANGE`].
    InvalidCountParam(usize),

    /// Elastic search error: {0}
    Elastic(#[source] reqwest::Error),

    /// Error receiving response: {0}
    Receiving(#[source] reqwest::Error),
}

impl Reject for Error {}

#[derive(Clone, Debug, Serialize)]
struct BaseError {
    request_id: Option<String>,
}

/// A unique identifier of a document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)]
pub(crate) struct DocumentId(pub(crate) String);

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PersonalizedDocumentData {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Similarity score of the personalized document.
    pub(crate) score: f32,

    /// Embedding from smbert.
    #[serde(skip_serializing)]
    pub(crate) embedding: Embedding,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,
}

impl AiDocument for PersonalizedDocumentData {
    type Id = DocumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.embedding
    }
}

/// Arbitrary properties that can be attached to a document.
pub type DocumentProperties = HashMap<String, serde_json::Value>;

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    pub(crate) count: Option<usize>,
}

/// Represents response from personalized documents endpoint.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    pub(crate) documents: Vec<PersonalizedDocumentData>,
}

impl PersonalizedDocumentsResponse {
    pub(crate) fn new(documents: impl Into<Vec<PersonalizedDocumentData>>) -> Self {
        Self {
            documents: documents.into(),
        }
    }

    pub(crate) fn to_reply(&self) -> impl Reply {
        reply::json(self)
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) enum PersonalizedDocumentsErrorKind {
    #[serde(rename = "not_enough_interactions")]
    NotEnoughInteractions,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct PersonalizedDocumentsError {
    #[serde(flatten)]
    base: BaseError,
    kind: PersonalizedDocumentsErrorKind,
}

impl PersonalizedDocumentsError {
    pub(crate) fn new(request_id: Option<String>, kind: PersonalizedDocumentsErrorKind) -> Self {
        Self {
            base: BaseError { request_id },
            kind,
        }
    }

    pub(crate) fn to_reply(&self, status: StatusCode) -> impl Reply {
        reply::with_status(reply::json(self), status)
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) enum UserInteractionType {
    #[serde(rename = "positive")]
    Positive = xayn_discovery_engine_core::document::UserReaction::Positive as isize,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UserInteractionData {
    #[serde(rename = "id")]
    pub(crate) document_id: DocumentId,
    #[serde(rename = "type")]
    pub(crate) interaction_type: UserInteractionType,
}

/// Represents user interaction request body.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UserInteractionRequestBody {
    pub(crate) documents: Vec<UserInteractionData>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub(crate) enum UserInteractionErrorKind {
    #[serde(rename = "invalid_user_id")]
    InvalidUserId,
    #[serde(rename = "invalid_document_id")]
    InvalidDocumentId,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct UserInteractionError {
    #[serde(flatten)]
    base: BaseError,
    kind: UserInteractionErrorKind,
}

impl UserInteractionError {
    pub(crate) fn new(request_id: Option<String>, kind: UserInteractionErrorKind) -> Self {
        Self {
            base: BaseError { request_id },
            kind,
        }
    }

    pub(crate) fn to_reply(&self, status: StatusCode) -> impl Reply {
        reply::with_status(reply::json(self), status)
    }
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)]
pub(crate) struct UserId(String);

impl UserId {
    pub(crate) fn new(id: impl AsRef<str>) -> Result<Self, Error> {
        let id = id.as_ref();

        if id.is_empty() {
            Err(Error::UserIdEmpty)
        } else if id.contains('\u{0000}') {
            Err(Error::UserIdContainsNul)
        } else {
            Ok(Self(id.to_string()))
        }
    }
}
