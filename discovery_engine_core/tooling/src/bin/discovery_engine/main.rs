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

use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use xayn_discovery_engine_ai::Embedding;
use xayn_discovery_engine_core::{document::UserReaction, stack};

fn deserialize_seconds_as_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(Duration::from_secs)
}

#[derive(Debug, Deserialize, PartialEq)]
struct Like {
    name: String,
    probability: f32,
    #[serde(deserialize_with = "deserialize_seconds_as_duration")]
    time_spent: Duration,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Dislike {
    name: String,
    probability: f32,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Persona {
    name: String,
    like_topics: Vec<Like>,
    dislike_topics: Vec<Dislike>,
    trusted_sources: Vec<String>,
    excluded_sources: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Input {
    num_iterations: usize,
    num_runs: usize,
    personas: Vec<Persona>,
    provider: String,
}

impl Input {
    fn read(path: impl AsRef<Path>) -> Result<Self> {
        serde_json::from_reader(BufReader::new(File::open(path)?)).map_err(Into::into)
    }
}

fn serialize_embedding_as_array<S>(embedding: &Embedding, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(embedding) = embedding.as_slice() {
        embedding.serialize(serializer)
    } else {
        embedding.to_vec().serialize(serializer)
    }
}

fn serialize_stack_id_as_name<S>(stack: &stack::Id, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    stack.name().serialize(serializer)
}

#[derive(Debug, Serialize)]
struct Document {
    topic: String,
    #[serde(serialize_with = "serialize_embedding_as_array")]
    embedding: Embedding,
    #[serde(serialize_with = "serialize_stack_id_as_name")]
    stack: stack::Id,
    user_reaction: UserReaction,
}

#[derive(Debug, Serialize)]
struct Output(Vec<Document>);

impl Output {
    fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        serde_json::to_writer(BufWriter::new(File::create(path)?), self).map_err(Into::into)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input() {
        let json = r#"
        {
            "num_iterations": 10,
            "num_runs": 5,
            "personas": [
                {
                    "name": "a",
                    "like_topics": [
                        {
                            "name": "art",
                            "probability": 0.5,
                            "time_spent": 30
                        }
                    ],
                    "dislike_topics": [
                        {
                            "name": "soccer",
                            "probability": 0.25
                        }
                    ],
                    "trusted_sources": [
                        "example.com"
                    ],
                    "excluded_sources": []
                }
            ],
            "provider": "mind"
        }
        "#;
        let input = serde_json::from_str::<Input>(json).unwrap();
        let expected = Input {
            num_iterations: 10,
            num_runs: 5,
            personas: vec![Persona {
                name: "a".into(),
                like_topics: vec![Like {
                    name: "art".into(),
                    probability: 0.5,
                    time_spent: Duration::from_secs(30),
                }],
                dislike_topics: vec![Dislike {
                    name: "soccer".into(),
                    probability: 0.25,
                }],
                trusted_sources: vec!["example.com".into()],
                excluded_sources: vec![],
            }],
            provider: "mind".into(),
        };
        assert_eq!(input, expected);
    }

    #[test]
    fn test_output() {
        let output = Output(vec![Document {
            topic: "art".into(),
            embedding: [1.0, 2.0, 3.0].into(),
            stack: stack::BreakingNews::id(),
            user_reaction: UserReaction::Positive,
        }]);
        let json = serde_json::to_string(&output).unwrap();
        let expected = r#"
        [
            {
                "topic": "art",
                "embedding": [1.0, 2.0, 3.0],
                "stack": "STACK_NAME",
                "user_reaction": 1
            }
        ]
        "#
        .replacen("STACK_NAME", stack::BreakingNews::name(), 1)
        .replace([' ', '\n'], "");
        assert_eq!(json, expected);
    }
}
