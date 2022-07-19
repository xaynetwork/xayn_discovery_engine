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

use chrono::{NaiveDateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use xayn_discovery_engine_ai::{Document as AiDocument, DocumentId, Embedding};
use xayn_discovery_engine_core::document::Id;

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Document {
    /// Unique identifier of the document.
    pub(crate) id: Id,

    /// Embedding from smbert.
    pub(crate) smbert_embedding: Embedding,

    /// Snippet of the resource.
    pub(crate) snippet: String,
}

impl Document {
    pub(crate) fn new((article, smbert_embedding): (Article, Embedding)) -> Self {
        Self {
            id: article.id.into(),
            snippet: article.description,
            smbert_embedding,
        }
    }
}

impl AiDocument for Document {
    fn id(&self) -> DocumentId {
        self.id.into()
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }

    fn date_published(&self) -> NaiveDateTime {
        Utc::now().naive_utc()
    }
}

/// Represents an article that is stored and loaded from local json file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Article {
    pub(crate) id: Uuid,
    pub(crate) description: String,
}

impl From<Document> for Article {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id.into(),
            description: doc.snippet,
        }
    }
}

/// Represents user interaction request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InteractionRequestBody {
    pub(crate) document_id: Id,
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display)]
pub(crate) struct UserId(Uuid);

impl FromStr for UserId {
    type Err = <Uuid as FromStr>::Err;

    fn from_str(user_id_str: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(user_id_str).map(Self)
    }
}
