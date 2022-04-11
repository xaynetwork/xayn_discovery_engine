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
    embedding::utils::COSINE_SIMILARITY_RANGE,
    error::Error,
};

// We need to re-export these, since they encapsulate the arguments
// required for pipeline construction, and are passed to builders.
pub use kpe::Config as KpeConfig;
pub use rubert::{QAMBertConfig, SMBertConfig};

#[cfg(test)]
mod tests;

// we need to export rstest_reuse from the root for it to work.
// `use rstest_reuse` will trigger `clippy::single_component_path_imports`
// which is not possible to silence.
#[cfg(test)]
#[allow(unused_imports)]
#[rustfmt::skip]
pub(crate) use rstest_reuse as rstest_reuse;
