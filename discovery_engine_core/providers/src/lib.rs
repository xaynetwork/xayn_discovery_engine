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
mod expression;
mod filter;
mod gnews;
pub mod gnews_client;
mod newscatcher;
pub mod newscatcher_client;
mod utils;

pub use bing::TrendingQuery;
pub use clean_query::clean_query;
pub use client::{default_from, Article, Client, Error, DEFAULT_WHEN};
pub use gnews_client::{HeadlinesQuery as GnewsHeadlinesQuery, NewsQuery as GnewsNewsQuery};
pub use newscatcher_client::{CommonQueryParts, HeadlinesQuery, NewsQuery, NewscatcherQuery};

pub use filter::{Filter, Market};

mod seal {
    pub trait Seal {}
}
