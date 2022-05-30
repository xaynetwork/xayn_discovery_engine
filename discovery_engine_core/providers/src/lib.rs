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

//! Providers.

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

use chrono::NaiveDateTime;
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod client;
mod expression;
mod filter;
pub mod gnews;
pub mod newscatcher;
mod utils;

pub use client::Client;
pub use gnews::{HeadlinesQuery as GnewsHeadlinesQuery, NewsQuery as GnewsNewsQuery};
pub use newscatcher::{
    default_from,
    CommonQueryParts,
    HeadlinesQuery,
    NewsQuery,
    NewscatcherQuery,
    DEFAULT_WHEN,
};

pub use filter::{Filter, Market};

mod seal {
    pub trait Seal {}
}

/// Client errors.
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Invalid API Url base
    InvalidUrlBase(Option<url::ParseError>),
    /// Failed to execute the HTTP request: {0}
    RequestExecution(#[source] reqwest::Error),
    /// Server returned a non-successful status code: {0}
    StatusCode(#[source] reqwest::Error),
    /// Failed to fetch from the server: {0}
    Fetching(#[source] reqwest::Error),
    /// Failed to decode the server's response: {0}
    Decoding(#[source] serde_json::Error),
    /// Failed to decode the server's response at JSON path {1}: {0}
    DecodingAtPath(
        String,
        #[source] serde_path_to_error::Error<serde_json::Error>,
    ),
}

/// A news article
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Article {
    /// Title of the resource.
    pub title: String,

    /// Snippet of the resource.
    pub snippet: String,

    /// Url to reach the resource.
    pub url: String,

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub source_domain: String,

    /// Publishing date.
    pub date_published: NaiveDateTime,

    /// Image attached to the news.
    pub image: String,

    /// The rank of the domain of the source,
    pub rank: u64,

    /// How much the article match the query.
    pub score: Option<f32>,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// Main topic of the publisher.
    pub topic: String,
}

impl Article {
    /// Gets the snippet or falls back to the title if the snippet is empty.
    pub fn snippet_or_title(&self) -> &str {
        (!self.snippet.is_empty())
            .then(|| &self.snippet)
            .unwrap_or(&self.title)
    }
}
