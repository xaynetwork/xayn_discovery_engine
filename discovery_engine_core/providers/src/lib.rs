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
    rust_2021_compatibility,
    unused_qualifications
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

mod client;
mod expression;
mod filter;
mod gnews;
mod gnews_client;
mod newscatcher;
mod newscatcher_client;
mod utils;

pub use client::{Article, Client, Error};
pub use gnews_client::NewsQuery as GnewsNewsQuery;
pub use newscatcher_client::{
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

#[cfg(feature = "direct_news_client_access")]
pub use crate::{
    gnews_client::Client as GnewsClient,
    newscatcher_client::Client as NewscatcherClient,
};
