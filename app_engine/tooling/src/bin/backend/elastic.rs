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

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct CountResponse {
    pub(crate) count: usize,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct Response<T> {
    pub(crate) hits: Hits<T>,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct Hits<T> {
    pub(crate) hits: Vec<Hit<T>>,
}

#[derive(Clone, Deserialize, Debug)]
pub(crate) struct Hit<T> {
    #[serde(rename = "_id")]
    pub(crate) id: String,
    #[serde(rename = "_source")]
    pub(crate) source: T,
    pub(crate) sort: Option<Value>,
}

/// An article in the MIND dataset.
#[derive(Clone, Deserialize, Debug)]
pub(crate) struct MindArticle {
    #[serde(rename = "Title")]
    pub(crate) title: String,
    #[serde(rename = "Abstract")]
    pub(crate) snippet: String,
    #[serde(rename = "URL")]
    pub(crate) url: String,
    #[serde(rename = "Category")]
    pub(crate) category: String,
    pub(crate) date_published: String,
}
