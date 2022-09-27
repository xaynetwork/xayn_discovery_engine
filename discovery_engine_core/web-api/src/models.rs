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

use derive_more::{AsRef, Deref, Display};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, string::FromUtf8Error};
use thiserror::Error;

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};

/// Web API errors.
#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum IdValidationError {
    /// Id can't be empty.
    Empty,

    /// Id can't contain NUL character.
    ContainsNul,

    /// Id is not a valid UTF-8 string. {0}
    InvalidUtf8(#[from] FromUtf8Error),
}

fn validate_id_from_string(id: impl AsRef<str>) -> Result<(), IdValidationError> {
    let id = id.as_ref();

    if id.is_empty() {
        return Err(IdValidationError::Empty);
    }
    if id.contains('\u{0000}') {
        return Err(IdValidationError::ContainsNul);
    }

    Ok(())
}

/// Unique document for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, Deref)]
pub(crate) struct DocumentId(String);

impl DocumentId {
    pub(crate) fn new(id: String) -> Result<Self, IdValidationError> {
        validate_id_from_string(&id)?;

        Ok(Self(id))
    }
}

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Document {
    /// Unique identifier of the document.
    pub(crate) id: DocumentId,

    /// Embedding from smbert.
    pub(crate) smbert_embedding: Embedding,

    /// Contents of the article.
    pub(crate) article: Article,
}

impl Document {
    pub(crate) fn new(id: DocumentId, article: Article, smbert_embedding: Embedding) -> Self {
        Self {
            id,
            smbert_embedding,
            article,
        }
    }
}

impl AiDocument for Document {
    type Id = DocumentId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }
}

/// Represents an article that is stored and loaded from local json file.
pub(crate) type Article = HashMap<String, serde_json::Value>;

impl From<Document> for Article {
    fn from(doc: Document) -> Self {
        doc.article
    }
}

/// Represents user interaction request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InteractionRequestBody {
    pub(crate) document_id: String,
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display, AsRef)]
pub(crate) struct UserId(String);

impl UserId {
    fn new(id: &str) -> Result<Self, IdValidationError> {
        let id = urlencoding::decode(id).map_err(IdValidationError::InvalidUtf8)?;

        validate_id_from_string(&*id)?;

        Ok(Self(id.into_owned()))
    }
}

impl FromStr for UserId {
    type Err = IdValidationError;

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
