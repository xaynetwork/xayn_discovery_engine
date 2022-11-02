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

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};

use crate::error::common::{InvalidDocumentId, InvalidDocumentPropertyId, InvalidUserId};

macro_rules! id_wrapper {
    ($name:ident, $validate:expr, $error:ident) => {
        /// A unique identifier of a document.
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
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String> + AsRef<str>) -> Result<Self, $error> {
                if ($validate)(id.as_ref()) {
                    Ok(Self(id.into()))
                } else {
                    Err($error { id: id.into() })
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

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }
    };
}

fn is_valid_id(id: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_\-:@.]+$").unwrap());
    RE.is_match(id)
}

id_wrapper!(DocumentId, is_valid_id, InvalidDocumentId);

id_wrapper!(DocumentPropertyId, is_valid_id, InvalidDocumentPropertyId);

id_wrapper!(UserId, is_valid_id, InvalidUserId);

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

    fn bert_embedding(&self) -> &Embedding {
        &self.embedding
    }
}

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    #[allow(dead_code)]
    pub(crate) count: Option<usize>,
}

/// Represents response from personalized documents endpoint.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    pub(crate) documents: Vec<PersonalizedDocumentData>,
}

impl PersonalizedDocumentsResponse {
    #[allow(dead_code)]
    pub(crate) fn new(documents: impl Into<Vec<PersonalizedDocumentData>>) -> Self {
        Self {
            documents: documents.into(),
        }
    }
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
