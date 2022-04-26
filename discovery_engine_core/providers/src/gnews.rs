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

use serde::{Deserialize, Serialize};

use crate::utils::deserialize_null_default;

/// A news article
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Article {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub title: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub description: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub content: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub url: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub image: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub source: Source,

    #[serde(rename(deserialize = "publishedAt"), alias="date_published")]
    pub published_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Source {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub name: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub url: String,
}

/// Query response from the Gnews API
#[derive(Deserialize, Debug)]
pub struct Response {
    #[serde(default)]
    pub articles: Vec<Article>,
    /// Total articles available
    #[serde(rename(deserialize = "totalArticles"))]
    pub total_articles: usize,
}
