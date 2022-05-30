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

use reqwest::Url;
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

    #[serde(rename(deserialize = "publishedAt"), alias = "date_published")]
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
pub(crate) struct Response {
    #[serde(default)]
    pub(crate) articles: Vec<Article>,

    /// Total articles available
    #[allow(dead_code)]
    #[serde(rename(deserialize = "totalArticles"))]
    pub(crate) total_articles: usize,
}

impl From<Article> for crate::Article {
    fn from(source: Article) -> Self {
        let source_domain = Url::parse(&source.url)
            .ok()
            .and_then(|url| url.domain().map(std::string::ToString::to_string))
            .unwrap_or_default();

        crate::Article {
            title: source.title,
            snippet: source.description,
            url: source.url,
            source_domain,
            date_published: source.published_at.naive_local(),
            image: source.image,
            rank: 0,
            score: None,
            country: String::new(),
            language: String::new(),
            topic: String::new(),
        }
    }
}
