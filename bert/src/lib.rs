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

//! The Bert pipelines compute embeddings of sequences.
//!
//! Sequences are anything string-like and can also be single words or snippets. The embeddings are
//! f32-arrays and their shape depends on the pooling strategy.
//!
//! See the example in this crate for usage details.

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

mod config;
mod embedding;
mod model;
mod pipeline;
mod pooler;
pub mod tokenizer;

pub use crate::{
    config::Config,
    embedding::{
        Embedding,
        Embedding1,
        Embedding2,
        InvalidEmbedding,
        NormalizedEmbedding,
        NormalizedSparseEmbedding,
        SparseEmbedding,
    },
    model::SparseModel,
    pipeline::{Pipeline, PipelineError},
    pooler::{AveragePooler, FirstPooler, NonePooler},
};

/// A Bert pipeline with an average pooler.
pub type AvgBert = Pipeline<crate::tokenizer::bert::Tokenizer, AveragePooler>;

/// A Roberta pipeline with an average pooler.
pub type AvgRoberta = Pipeline<crate::tokenizer::roberta::Tokenizer, AveragePooler>;
