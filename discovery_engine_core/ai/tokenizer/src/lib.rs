//! A Bert tokenizer which converts sequences into encodings.
//!
//! This is a very condensed and heavily refactored version of [huggingface's `tokenizers`] crate.
//!
//! The tokenizer is based on a word piece vocabulary and consists of a Bert normalizer, a Bert
//! pre-tokenizer, a Bert word piece model and a Bert post-tokenizer including truncation and
//! padding strategies. The encodings can be of any numerical data type which implements
//! [`Num`]` + `[`FromPrimitive`]` + `[`Copy`].
//!
//! The normalizer is configurable by:
//! - Cleans any control characters and replaces all sorts of whitespace by ` `.
//! - Separates Chinese characters by whitespace so they get split.
//! - Keeps accents of characters.
//! - Lowercases characters.
//!
//! The pre-tokenizer is not configurable.
//!
//! The word piece model is configurable by:
//! - The unknown token.
//! - The continuing subword prefix.
//! - The maximum number of characters per word.
//!
//! The post-tokenizer is configurable by:
//! - The class token.
//! - The separation token.
//! - A truncation strategy.
//! - A padding strategy.
//!
//! ```no_run
//! use xayn_discovery_engine_tokenizer::{
//!     AccentChars,
//!     Builder,
//!     CaseChars,
//!     ChineseChars,
//!     ControlChars,
//!     Padding,
//!     Truncation,
//! };
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let tokenizer = Builder::<u32>::from_file("vocab.txt")?
//!         .with_normalizer(
//!             ControlChars::Cleanse,
//!             ChineseChars::Separate,
//!             AccentChars::Cleanse,
//!             CaseChars::Lower,
//!         )
//!         .with_model("[UNK]", "##", 100)
//!         .with_post_tokenizer("[CLS]", "[SEP]")
//!         .with_truncation(Truncation::fixed(128, 0))
//!         .with_padding(Padding::fixed(128, "[PAD]"))
//!         .build()?;
//!
//!     let encoding = tokenizer.encode("This îs ã séquènce.");
//!     assert_eq!(tokenizer.decode(&encoding, true), "this is a sequence.");
//!
//!     Ok(())
//! }
//! ```
//!
//! [huggingface's `tokenizers`]: https://crates.io/crates/tokenizers
//! [`Num`]: num_traits::Num
//! [`FromPrimitive`]: num_traits::FromPrimitive

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

mod builder;
mod model;
mod normalizer;
mod post_tokenizer;
mod pre_tokenizer;
mod tokenizer;

pub use crate::{
    builder::{Builder, BuilderError},
    model::ModelError,
    normalizer::{string::Offsets, AccentChars, CaseChars, ChineseChars, ControlChars},
    post_tokenizer::{
        encoding::Encoding,
        padding::{Padding, PaddingError},
        truncation::{Truncation, TruncationError},
        PostTokenizerError,
    },
    tokenizer::Tokenizer,
};

/// A stack allocated string with a maximum length of eight bytes.
type SmallString = smallstr::SmallString<[u8; 8]>;
