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
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::Document as AiDocument;

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
pub(crate) struct DocumentProperty(serde_json::Value);

/// Arbitrary properties that can be attached to a document.
pub(crate) type DocumentProperties = HashMap<DocumentPropertyId, DocumentProperty>;

/// Represents a result from an interaction query.
#[derive(Clone, Debug)]
pub(crate) struct InteractedDocument {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Embedding from smbert.
    pub(crate) embedding: NormalizedEmbedding,

    /// The tags associated to the document.
    pub(crate) tags: Vec<String>,
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
    pub(crate) tags: Vec<String>,
}

impl AiDocument for PersonalizedDocument {
    type Id = DocumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn bert_embedding(&self) -> &NormalizedEmbedding {
        &self.embedding
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub(crate) enum UserInteractionType {
    Positive = 1,
}

/// Represents a document sent for ingestion.
#[derive(Clone, Debug)]
pub(crate) struct IngestedDocument {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Snippet used to calculate embeddings for a document.
    pub(crate) snippet: String,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,

    /// The tags associated to the document.
    pub(crate) tags: Vec<String>,
}
