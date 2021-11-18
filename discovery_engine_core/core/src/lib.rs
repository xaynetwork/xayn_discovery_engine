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
<<<<<<< HEAD
=======

mod document;
mod engine;
mod error;

pub use crate::{
    document::{Document, DocumentId, Embedding, Embedding1},
    engine::{DiscoveryEngine, InternalState, Stack},
    error::Error,
};
>>>>>>> added DiscoveryEngine type
