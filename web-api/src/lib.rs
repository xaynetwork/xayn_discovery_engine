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

//! The web engine & api.

#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]
#![deny(
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications,
    unsafe_op_in_unsafe_fn
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod app;
pub mod config;
mod embedding;
mod error;
pub mod extractor;
mod ingestion;
pub mod logging;
mod middleware;
#[cfg(test)]
mod mind;
mod models;
mod net;
mod personalization;
pub mod rank_merge;
mod storage;
mod tenants;
mod utils;

pub use crate::{
    app::{start, Application, SetupError},
    error::application::{ApplicationError, Error},
    ingestion::Ingestion,
    net::AppHandle,
    personalization::{bench_derive_interests, bench_rerank, Personalization},
};

/// Allow migration tests to have access to the elastic search mapping this uses.
//FIXME: Remove once we only test migrations upward from a version with `web-api-db-ctrl`
pub static ELASTIC_MAPPING: &str = include_str!("../../web-api-db-ctrl/elasticsearch/mapping.json");
