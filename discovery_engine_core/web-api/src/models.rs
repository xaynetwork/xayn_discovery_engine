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

use derive_more::{AsRef, Display};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, string::FromUtf8Error};
use thiserror::Error;

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};

/// Web API errors.
#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum Error {
    /// [`UserId`] can't be empty.
    UserIdEmpty,

    /// [`UserId`] can't contain NUL character.
    UserIdContainsNul,

    /// Failed to decode [`UserId] from path param: {0}.
    UserIdUtf8Conversion(#[from] FromUtf8Error),

    /// Elastic search error: {0}
    Elastic(#[source] reqwest::Error),

    /// Error receiving response: {0}
    Receiving(#[source] reqwest::Error),
}

/// A unique identifier of a document.
#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, Display)]
pub(crate) struct DocumentId(pub(crate) String);

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PersonalizedDocument {
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

impl PersonalizedDocument {
    pub(crate) fn new((ingested_doc, embedding): (IngestedDocument, Embedding)) -> Self {
        Self {
            id: DocumentId(ingested_doc.id),
            score: 0.0,
            embedding,
            properties: ingested_doc.properties,
        }
    }
}

impl AiDocument for PersonalizedDocument {
    type Id = DocumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.embedding
    }
}

/// Represents a document sent for ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IngestedDocument {
    /// Unique identifier of the document.
    pub(crate) id: String,

    /// Snippet used to calculate embeddings for a document.
    pub(crate) snippet: String,

    /// Contents of the document properties.
    pub(crate) properties: DocumentProperties,
}

/// Arbitrary properties that can be attached to a document.
pub(crate) type DocumentProperties = HashMap<String, serde_json::Value>;

/// Represents user interaction request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InteractionRequestBody {
    pub(crate) document_id: String,
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)]
pub(crate) struct UserId(String);

impl UserId {
    fn new(value: &str) -> Result<Self, Error> {
        let value = urlencoding::decode(value).map_err(Error::UserIdUtf8Conversion)?;

        if value.trim().is_empty() {
            Err(Error::UserIdEmpty)
        } else if value.contains('\u{0000}') {
            Err(Error::UserIdContainsNul)
        } else {
            Ok(Self(value.to_string()))
        }
    }
}

impl FromStr for UserId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        UserId::new(value)
    }
}

#[repr(u8)]
pub(crate) enum UserReaction {
    Positive = xayn_discovery_engine_core::document::UserReaction::Positive as u8,
    #[allow(dead_code)]
    Negative = xayn_discovery_engine_core::document::UserReaction::Negative as u8,
}
