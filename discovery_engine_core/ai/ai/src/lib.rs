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
mod embedding;
mod error;
mod ranker;
mod utils;

pub use crate::{
    coi::{
        config::{Config as CoiSystemConfig, Error as CoiSystemConfigError},
        key_phrase::KeyPhrase,
        point::{CoiPoint, NegativeCoi, PositiveCoi},
        CoiId,
    },
    embedding::{
        utils::{cosine_similarity, pairwise_cosine_similarity, COSINE_SIMILARITY_RANGE},
        Embedding,
    },
    error::Error,
    ranker::{
        document::{Document, DocumentId, UserFeedback},
        public::{Builder, Ranker},
    },
};

// we need to export rstest_reuse from the root for it to work.
// `use rstest_reuse` will trigger `clippy::single_component_path_imports`
// which is not possible to silence.
#[cfg(test)]
#[allow(unused_imports)]
#[rustfmt::skip]
pub(crate) use rstest_reuse as rstest_reuse;
