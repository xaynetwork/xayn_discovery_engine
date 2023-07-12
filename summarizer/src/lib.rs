// Copyright 2023 Xayn AG
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

//! Summarize a large text. It uses a probabilistic approach.

#![forbid(unsafe_op_in_unsafe_fn)]
#![forbid(unsafe_code)]
#![deny(
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

mod summarizers;

/// Summarizes a source to the amount of sentences specified in the `config`.
///
/// Currently we have 2 `summarizer` implementations, naive and rank based.
/// Naive is for now the preferred approach. We keep rank based for testing purposes.
///
/// `source` can be either an url, html or just plain text.
///
/// When using url, we will attempt to fetch the content first.
/// When using html or url, the content will first pass-through a readability implementation from Mozilla.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use xayn_summarizer::{summarize, Config, Source, Summarizer};
///
/// let summary = summarize(
///     &Summarizer::Naive,
///     &Source::PlainText {
///         text: "Lorem ispum dolor si amet...".to_string(),
///     },
///     &Config::default(),
/// );
/// ```
pub fn summarize(summarizer: &Summarizer, source: &Source, config: &Config) -> String {
    let text = source.to_readable_text();
    let summary = match summarizer {
        Summarizer::Naive => summarizers::naive::summarize(&text, config.num_sentences),
        Summarizer::RankBased => {
            summarizers::rank_based::summarize(&text, &[], config.num_sentences)
        }
    };

    if summary.is_empty() {
        text
    } else {
        summary
    }
}

/// Configures how many sentences should be kept, from the original source.
/// Defaults to 4 sentences.
#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub num_sentences: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config { num_sentences: 4 }
    }
}

/// Specifies where the content that needs to be summarized is located.
/// Which is either remote (url), local (html or plain text).
pub enum Source {
    PlainText { text: String },
}

impl Source {
    fn to_readable_text(&self) -> String {
        match self {
            Source::PlainText { text } => text.clone(),
        }
    }
}

/// The summarizer implementations we are currently supporting.
pub enum Summarizer {
    Naive,
    RankBased,
}
