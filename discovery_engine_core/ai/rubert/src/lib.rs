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

#![cfg_attr(
    doc,
    forbid(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![forbid(unsafe_op_in_unsafe_fn)]
//! The RuBert pipeline computes embeddings of sequences.
//!
//! Sequences are anything string-like and can also be single words or snippets. The embeddings are
//! f32-arrays and their shape depends on the pooling strategy.
//!
//! See the example in this crate for usage details.

mod config;
mod model;
mod pipeline;
mod pooler;
mod tokenizer;

pub use crate::{
    config::{Config, ConfigError},
    model::kinds,
    pipeline::{Pipeline, PipelineError},
    pooler::{
        ArcEmbedding1,
        ArcEmbedding2,
        AveragePooler,
        Embedding1,
        Embedding2,
        FirstPooler,
        NonePooler,
    },
};

/// A sentence (embedding) multilingual Bert pipeline.
#[allow(clippy::upper_case_acronyms)]
pub type SMBert = Pipeline<kinds::SMBert, AveragePooler>;
pub type SMBertConfig<'a, P> = Config<'a, kinds::SMBert, P>;

/// A question answering (embedding) multilingual Bert pipeline.
#[allow(clippy::upper_case_acronyms)]
pub type QAMBert = Pipeline<kinds::QAMBert, AveragePooler>;
pub type QAMBertConfig<'a, P> = Config<'a, kinds::QAMBert, P>;

#[cfg(doc)]
pub use crate::{
    model::{BertModel, ModelError},
    pooler::{Embedding, PoolerError},
    tokenizer::TokenizerError,
};
