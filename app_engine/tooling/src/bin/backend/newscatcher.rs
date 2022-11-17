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
pub(crate) struct Article {
    pub(crate) title: String,
    #[serde(rename(serialize = "_score"), skip_serializing_if = "Option::is_none")]
    pub(crate) score: Option<f32>,
    pub(crate) rank: u64,
    pub(crate) clean_url: String,
    pub(crate) excerpt: String,
    pub(crate) link: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) media: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) topic: String,
    pub(crate) country: String,
    pub(crate) language: String,
    pub(crate) published_date: String,
    pub(crate) embedding: Vec<f32>,
}

#[derive(Serialize, Debug)]
pub(crate) struct Response {
    pub(crate) status: String,
    pub(crate) articles: Vec<Article>,
    pub(crate) total_pages: usize,
}

impl Response {
    pub(crate) fn new(articles: Vec<Article>, total_pages: usize) -> Self {
        Self {
            status: "ok".to_string(),
            articles,
            total_pages,
        }
    }
}
