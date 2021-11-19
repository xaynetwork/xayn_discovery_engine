//! Xayn Discovery Engine core.
#![warn(missing_docs, unreachable_pub)]
#![forbid(
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    // unused_qualifications
)]
#![deny(clippy::pedantic)]
#![deny(clippy::future_not_send)]

mod document;
mod engine;

pub use crate::{
    document::{Document, DocumentId, Embedding, Embedding1},
    engine::{DiscoveryEngine, InternalState, Stack},
};
