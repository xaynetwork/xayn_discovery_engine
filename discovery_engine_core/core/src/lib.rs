//! Xayn Discovery Engine core.
#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused_qualifications
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

mod document;
mod engine;
mod mab;
/// Export types to customize the behaviour of a stack.
pub mod stack;
mod utils;

pub use crate::{
    document::{Document, Embedding1, Id},
    engine::Engine,
};
