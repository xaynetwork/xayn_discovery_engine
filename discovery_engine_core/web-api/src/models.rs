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
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

use xayn_discovery_engine_ai::{Document as AiDocument, DocumentId, Embedding};
use xayn_discovery_engine_core::document::Id;

/// Web API errors.
#[derive(Error, Debug, DisplayDoc)]
pub(crate) enum Error {
    /// [`UserId`] can't be empty.
    EmptyUserId,
    /// [`ProviderId`] can't be empty.
    EmptyProviderId,
}

/// Unique identifier of the document from the provider.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display)]
pub(crate) struct ProviderId(String);

impl ProviderId {
    fn new(value: &str) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::EmptyProviderId)
        } else {
            Ok(Self(value.to_string()))
        }
    }
}

impl TryFrom<&str> for ProviderId {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        ProviderId::new(value)
    }
}

/// Represents a result from a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Document {
    /// Unique identifier of the document.
    pub(crate) id: Id,

    /// Identifier of the document from the provider.
    pub(crate) provider_id: ProviderId,

    /// Embedding from smbert.
    pub(crate) smbert_embedding: Embedding,

    /// Contents of the article.
    pub(crate) article: Article,
}

impl Document {
    pub(crate) fn new(
        (provider_id, article, smbert_embedding): (ProviderId, Article, Embedding),
    ) -> Self {
        let id = Uuid::new_v4().into();
        Self {
            id,
            provider_id,
            article,
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
pub(crate) type Article = HashMap<String, serde_json::Value>;

impl From<Document> for Article {
    fn from(doc: Document) -> Self {
        doc.article
    }
}

/// Represents user interaction request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InteractionRequestBody {
    pub(crate) document_id: ProviderId,
}

/// Unique identifier for the user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display)]
pub(crate) struct UserId(String);

impl UserId {
    fn new(value: &str) -> Result<Self, Error> {
        if value.is_empty() {
            Err(Error::EmptyUserId)
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
