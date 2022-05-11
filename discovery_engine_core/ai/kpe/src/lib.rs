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

//! The KPE pipeline extracts key phrases from a sequence.
//!
//! See `examples/` for a usage example.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications,
    unsafe_code
)]
#![warn(missing_docs)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::items_after_statements
)]

mod config;
mod model;
mod pipeline;
mod tokenizer;

pub use crate::{
    config::{Config, ConfigError},
    pipeline::{Pipeline, PipelineError},
    tokenizer::key_phrase::RankedKeyPhrases,
};

#[cfg(doc)]
pub use crate::{model::ModelError, tokenizer::TokenizerError};
