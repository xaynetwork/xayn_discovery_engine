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

mod engine;
mod io;

use anyhow::Result;
use clap::Parser;
use xayn_discovery_engine::{document::UserReaction, stack};

use crate::{
    engine::TestEngine,
    io::{Document, Input, Output},
};

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
    /// Pretty prints the JSON output.
    #[clap(action, long)]
    pretty: bool,
    /// Displays engine progress to stderr.
    #[clap(action, long)]
    progress: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let input = Input::read(args.input)?;
    let output = if input.provider == "dummy" {
        Output(
            [(
                "a".into(),
                vec![Document {
                    topic: "art".into(),
                    embedding: [1.0, 2.0, 3.0].into(),
                    stack: stack::BreakingNews::id(),
                    user_reaction: UserReaction::Positive,
                }],
            )]
            .into(),
        )
    } else {
        TestEngine::new(input.provider, args.progress)
            .await?
            .run(input.num_runs, input.num_iterations, input.personas)
            .await?
    };

    output.write(args.output, args.pretty)
}
