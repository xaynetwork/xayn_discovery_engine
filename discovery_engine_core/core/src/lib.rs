// Copyright 2021 Xayn AG
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

//! Xayn Discovery Engine core.

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

mod config;
pub mod document;
mod engine;
mod mab;
pub mod stack;
mod state;
#[cfg(feature = "storage")]
pub mod storage;
//FIXME they are not storage specific but currently only used by storage
#[cfg(feature = "storage")]
mod utils;

pub use crate::{
    config::{CoreConfig, EndpointConfig, ExplorationConfig, FeedConfig, InitConfig, SearchConfig},
    engine::{Engine, Error, SearchBy},
};

//FIXME move into crate::storage once the feature "storage" flag is removed
pub struct DartMigrationData {
    pub dummy: u8,
}

//FIXME move into crate::storage once the feature "storage" flag is removed
/// Hint about what was done during db init.
pub enum InitDbHint {
    /// Hint to use if nothing special happened during init.
    NormalInit,
    /// A new db was created, there was no db beforehand.
    #[cfg(feature = "storage")]
    NewDbCreated,
    /// There was a db but we could not open it so we deleted it and created a new one.
    #[cfg(feature = "storage")]
    DbOverwrittenDueToErrors(crate::storage::Error),
}
