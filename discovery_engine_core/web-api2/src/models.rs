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

use std::{borrow::Cow, collections::HashMap, str::FromStr};

use derive_more::{AsRef, Display};
use serde::{Deserialize, Serialize};

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};

use crate::error::common::{InvalidDocumentId, InvalidPropertyId, InvalidUserId};

/// A unique identifier of a document.
#[derive(
    AsRef,
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
pub struct DocumentId(String);

impl DocumentId {
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidDocumentId> {
        let id = id.into();

        if id.is_empty() || id.contains('\u{0000}') {
            Err(InvalidDocumentId)
        } else {
            Ok(Self(id))
        }
    }

    pub fn encode(&self) -> Cow<str> {
        urlencoding::encode(self.as_ref())
    }
}

impl FromStr for DocumentId {
    type Err = InvalidDocumentId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl From<DocumentId> for String {
    fn from(item: DocumentId) -> Self {
        item.0
    }
}

#[derive(Clone, Debug, Display, Serialize, Deserialize, PartialEq, Eq, Hash, AsRef)]
pub struct DocumentPropertyId(String);

impl DocumentPropertyId {
    #[allow(dead_code)]
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidPropertyId> {
        let id = id.into();

        if id.is_empty() || id.contains('\u{0000}') {
            Err(InvalidPropertyId)
        } else {
            Ok(Self(id))
        }
    }

    #[allow(dead_code)]
    pub fn encode(&self) -> Cow<str> {
        urlencoding::encode(self.as_ref())
    }
}

impl FromStr for DocumentPropertyId {
    type Err = InvalidPropertyId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DocumentProperty(serde_json::Value);

/// Arbitrary properties that can be attached to a document.
pub type DocumentProperties = HashMap<DocumentPropertyId, DocumentProperty>;

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

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    #[allow(dead_code)]
    pub(crate) count: Option<usize>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) enum UserInteractionType {
    #[serde(rename = "positive")]
    Positive = 1,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UserInteractionData {
    #[allow(dead_code)]
    #[serde(rename = "id")]
    pub(crate) document_id: DocumentId,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub(crate) interaction_type: UserInteractionType,
}

/// Represents user interaction request body.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UserInteractionRequestBody {
    #[allow(dead_code)]
    pub(crate) documents: Vec<UserInteractionData>,
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)]
pub struct UserId(String);

impl UserId {
    #[allow(dead_code)]
    pub(crate) fn new(id: impl AsRef<str>) -> Result<Self, InvalidUserId> {
        let id = id.as_ref();

        if id.is_empty() || id.contains('\u{0000}') {
            Err(InvalidUserId)
        } else {
            Ok(Self(id.to_string()))
        }
    }
}

impl FromStr for UserId {
    type Err = InvalidUserId;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}
