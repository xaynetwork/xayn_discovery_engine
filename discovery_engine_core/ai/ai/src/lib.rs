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

//! The AI of the discovery engine.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unsafe_code,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod coi;
mod document;
mod embedding;
mod error;
mod kps;
pub mod utils;

pub use crate::{
    coi::{
        config::{Config as CoiConfig, Error as CoiConfigError},
        point::{CoiPoint, NegativeCoi, PositiveCoi, UserInterests},
        stats::CoiStats,
        system::System as CoiSystem,
        CoiId,
    },
    document::{Document, DocumentId},
    embedding::{
        cosine_similarity,
        pairwise_cosine_similarity,
        Embedding,
        MalformedBytesEmbedding,
    },
    error::GenericError,
    kps::{
        config::{Config as KpsConfig, Error as KpsConfigError},
        key_phrase::{KeyPhrase, KeyPhrases},
        system::System as KpsSystem,
    },
    utils::{nan_safe_f32_cmp, nan_safe_f32_cmp_desc},
};

#[cfg(doc)]
pub use crate::embedding::COSINE_SIMILARITY_RANGE;
