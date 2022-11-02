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

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
    time::Duration,
};

use anyhow::Result;
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
pub(crate) struct Like {
    pub(crate) probability: f64,
    #[serde(deserialize_with = "deserialize_seconds_as_duration")]
    pub(crate) time_spent: Duration,
}

pub(crate) type Likes = HashMap<String, Like>;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct Dislike {
    pub(crate) probability: f64,
}

pub(crate) type Dislikes = HashMap<String, Dislike>;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct Persona {
    pub(crate) like_topics: Likes,
    pub(crate) dislike_topics: Dislikes,
    pub(crate) trusted_sources: Vec<String>,
    pub(crate) excluded_sources: Vec<String>,
}

pub(crate) type Personas = HashMap<String, Persona>;

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct Input {
    pub(crate) num_runs: usize,
    pub(crate) num_iterations: usize,
    pub(crate) personas: Personas,
    pub(crate) provider: String,
}

impl Input {
    pub(crate) fn read(path: impl AsRef<Path>) -> Result<Self> {
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
pub(crate) struct Document {
    pub(crate) topic: String,
    #[serde(serialize_with = "serialize_embedding_as_array")]
    pub(crate) embedding: Embedding,
    #[serde(serialize_with = "serialize_stack_id_as_name")]
    pub(crate) stack: stack::Id,
    pub(crate) user_reaction: UserReaction,
}

impl From<xayn_discovery_engine_core::document::Document> for Document {
    fn from(document: xayn_discovery_engine_core::document::Document) -> Self {
        Self {
            topic: document.resource.topic,
            embedding: document.bert_embedding,
            stack: document.stack_id,
            user_reaction: document.reaction.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Output(pub(crate) HashMap<String, Vec<Document>>);

impl Output {
    pub(crate) fn write(&self, path: impl AsRef<Path>, pretty: bool) -> Result<()> {
        let writer = BufWriter::new(File::create(path)?);
        if pretty {
            serde_json::to_writer_pretty(writer, self)
        } else {
            serde_json::to_writer(writer, self)
        }
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input() {
        let json = r#"
        {
            "num_runs": 2,
            "num_iterations": 5,
            "personas": {
                "a": {
                    "like_topics": {
                        "art": {
                            "probability": 0.5,
                            "time_spent": 30
                        }
                    },
                    "dislike_topics": {
                        "soccer": {
                            "probability": 0.25
                        }
                    },
                    "trusted_sources": [
                        "example.com"
                    ],
                    "excluded_sources": []
                }
            },
            "provider": "dummy"
        }
        "#;
        let input = serde_json::from_str::<Input>(json).unwrap();
        let expected = Input {
            num_runs: 2,
            num_iterations: 5,
            personas: [(
                "a".into(),
                Persona {
                    like_topics: [(
                        "art".into(),
                        Like {
                            probability: 0.5,
                            time_spent: Duration::from_secs(30),
                        },
                    )]
                    .into(),
                    dislike_topics: [("soccer".into(), Dislike { probability: 0.25 })].into(),
                    trusted_sources: vec!["example.com".into()],
                    excluded_sources: vec![],
                },
            )]
            .into(),
            provider: "dummy".into(),
        };
        assert_eq!(input, expected);
    }

    #[test]
    fn test_output() {
        let output = Output(
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
        );
        let json = serde_json::to_string(&output).unwrap();
        let expected = r#"
        {
            "a": [
                {
                    "topic": "art",
                    "embedding": [1.0, 2.0, 3.0],
                    "stack": "STACK_NAME",
                    "user_reaction": 1
                }
            ]
        }
        "#
        .replacen("STACK_NAME", stack::BreakingNews::name(), 1)
        .replace([' ', '\n'], "");
        assert_eq!(json, expected);
    }
}
