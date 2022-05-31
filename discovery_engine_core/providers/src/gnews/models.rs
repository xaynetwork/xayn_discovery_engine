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

use crate::{utils::deserialize_null_default, Market};

/// A news article
#[derive(Clone, Serialize, Deserialize, Debug)]
pub(super) struct Article {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) title: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) description: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) content: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) url: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) image: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) source: Source,

    #[serde(rename(deserialize = "publishedAt"), alias = "date_published")]
    pub(super) published_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub(super) struct Source {
    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) name: String,

    #[serde(deserialize_with = "deserialize_null_default")]
    pub(super) url: String,
}

/// Query response from the Gnews API
#[derive(Deserialize, Debug)]
pub(super) struct Response {
    #[serde(default)]
    pub(super) articles: Vec<Article>,

    /// Total articles available
    #[allow(dead_code)]
    #[serde(rename(deserialize = "totalArticles"))]
    pub(super) total_articles: usize,
}

impl Article {
    pub(super) fn into_generic_article(self, market: Market) -> crate::Article {
        let source_domain = Url::parse(&self.url)
            .ok()
            .and_then(|url| url.domain().map(std::string::ToString::to_string))
            .unwrap_or_default();

        crate::Article {
            title: self.title,
            snippet: self.description,
            url: self.url,
            source_domain,
            date_published: self.published_at.naive_local(),
            image: self.image,
            rank: 0,
            score: None,
            market,
            topic: String::new(),
        }
    }
}
