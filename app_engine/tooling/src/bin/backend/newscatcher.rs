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

// Here we emulate the format of the Newscatcher API, so that it's compatible with
// our current client in the discovery engine.

use serde::Serialize;

#[derive(Clone, Serialize, Debug)]
pub struct Article {
    pub title: String,
    #[serde(rename(serialize = "_score"), skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    pub rank: u64,
    pub clean_url: String,
    pub excerpt: String,
    pub link: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub media: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub topic: String,
    pub country: String,
    pub language: String,
    pub published_date: String,
    pub embedding: Vec<f32>,
}

#[derive(Serialize, Debug)]
pub struct Response {
    pub status: String,
    pub articles: Vec<Article>,
    pub total_pages: usize,
}

impl Response {
    pub fn new(articles: Vec<Article>, total_pages: usize) -> Self {
        Self {
            status: "ok".to_string(),
            articles,
            total_pages,
        }
    }
}
