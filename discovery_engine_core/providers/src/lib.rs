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
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod bing;
mod clean_query;
mod client;
mod error;
mod expression;
mod filter;
mod models;
mod newscatcher;

pub use models::{GenericArticle, Rank, UrlWithDomain};

pub use bing::{TrendingQuery, TrendingTopic};
pub use clean_query::clean_query;
pub use client::{Client, CommonQueryParts, HeadlinesQuery, NewsQuery, Query, RankLimit};
pub use error::Error;
pub use filter::{Filter, Market};
pub use newscatcher::{Article as NewscatcherArticle, Response as NewscatcherResponse};

mod seal {
    pub trait Seal {}
}
