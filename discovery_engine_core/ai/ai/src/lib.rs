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

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(clippy::pedantic, unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::items_after_statements
)]

mod coi;
mod data;
mod embedding;
mod error;
pub mod ranker;
mod utils;

pub use crate::{
    coi::CoiId,
    data::document::{
        DayOfWeek,
        Document,
        DocumentHistory,
        DocumentId,
        QueryId,
        Relevance,
        SessionId,
        UserAction,
        UserFeedback,
    },
    embedding::utils::{cosine_similarity, COSINE_SIMILARITY_RANGE},
    error::Error,
};

// we need to export rstest_reuse from the root for it to work.
// `use rstest_reuse` will trigger `clippy::single_component_path_imports`
// which is not possible to silence.
#[cfg(test)]
#[allow(unused_imports)]
#[rustfmt::skip]
pub(crate) use rstest_reuse as rstest_reuse;
