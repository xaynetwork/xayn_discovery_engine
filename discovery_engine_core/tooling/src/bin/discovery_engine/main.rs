// Copyright 2022 Xayn AG
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

//! Discovery engine for end-to-end tests.

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

mod io;

use anyhow::Result;
use clap::Parser;
use xayn_discovery_engine_core::{document::UserReaction, stack};

use crate::io::{Document, Input, Output};

/// Discovery engine for end-to-end tests.
#[derive(Debug, Parser)]
#[clap(name = "Discovery Engine E2E", version)]
struct Args {
    /// Path to the input JSON file.
    #[clap(long, short)]
    input: String,
    /// Path to the output JSON file.
    #[clap(long, short)]
    output: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    Input::read(args.input)?;
    Output(vec![Document {
        topic: "art".into(),
        embedding: [1.0, 2.0, 3.0].into(),
        stack: stack::BreakingNews::id(),
        user_reaction: UserReaction::Positive,
    }])
    .write(args.output)?;

    Ok(())
}
