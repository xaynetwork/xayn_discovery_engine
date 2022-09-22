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

use chrono::{DateTime, Utc};
use derive_more::{AsRef, Display};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, string::FromUtf8Error};
use thiserror::Error;
use uuid::Uuid;

use xayn_discovery_engine_ai::{Document as AiDocument, Embedding};
use xayn_discovery_engine_core::document::Id;

/// Web API errors.
#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum Error {
    /// [`UserId`] can't be empty.
    UserIdEmpty,

    /// [`UserId`] can't contain NUL character.
    UserIdContainsNul,

    /// Failed to decode [`UserId] from path param: {0}.
    UserIdUtf8Conversion(#[from] FromUtf8Error),
}

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Document {
    /// Unique identifier of the document.
    pub(crate) id: Id,

    /// Embedding from smbert.
    pub(crate) smbert_embedding: Embedding,

    /// Contents of the article.
    pub(crate) article: Article,
}

impl Document {
    pub(crate) fn new((article, smbert_embedding): (Article, Embedding)) -> Self {
        let id = Uuid::new_v4().into();
        Self {
            id,
            smbert_embedding,
            article,
        }
    }
}

impl AiDocument for Document {
    type Id = Id;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }

    fn date_published(&self) -> DateTime<Utc> {
        Utc::now()
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
