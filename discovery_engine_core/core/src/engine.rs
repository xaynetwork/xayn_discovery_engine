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

use std::collections::HashMap;

use displaydoc::Display;
use futures::stream::{FuturesUnordered, StreamExt, TryStreamExt};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::{
    document::{Document, TimeSpent, UserReacted},
    mab::{self, BetaSampler, Selection},
    ranker::Ranker,
    stack::{self, BoxedOps, Data as StackData, Id as StackId, Stack},
};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to serialize internal state of the engine: {0}.
    Serialization(#[source] bincode::Error),

    /// Failed to deserialize internal state to create the engine: {0}.
    Deserialization(#[source] bincode::Error),

    /// No operations on stack were provided.
    NoStackOps,

    /// Invalid stack: {0}.
    InvalidStack(#[source] stack::Error),

    /// Invalid stack id: {0}.
    InvalidStackId(StackId),

    /// An operation on a stack failed: {0}.
    StackOpFailed(#[source] stack::Error),

    /// Error while selecting the documents to return: {0}.
    Selection(#[from] mab::Error),

    /// Error while using the ranker.
    Ranker(#[from] GenericError),

    /// A list of errors that could occur during some operation.
    Errors(Vec<Error>),
}

#[allow(dead_code)]
/// Feed market.
struct Market {
    country_code: String,
    lang_code: String,
}

#[allow(dead_code)]
/// Discovery Engine configuration settings.
pub struct Config {
    api_key: String,
    api_base_url: String,
    markets: Vec<Market>,
    smbert_vocab: String,
    smbert_model: String,
    kpe_vocab: String,
    kpe_model: String,
    kpe_cnn: String,
    kpe_classifier: String,
}

/// Discovery Engine.
pub struct Engine<R> {
    #[allow(dead_code)]
    config: Config,
    stacks: HashMap<StackId, RwLock<Stack>>,
    ranker: R,
}

impl<R> Engine<R>
where
    R: Ranker + Send + Sync,
{
    /// Creates a new `Engine` from configuration.
    pub fn from_config(config: Config, ranker: R, stack_ops: Vec<BoxedOps>) -> Result<Self, Error> {
        let stacks = stack_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                Stack::new(StackData::default(), ops).map(|stack| (id, stack))
            })
            .collect::<Result<_, _>>()
            .map_err(Error::InvalidStack)?;

        Ok(Self {
            config,
            stacks,
            ranker,
        })
    }

    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    pub fn new(
        state: &[u8],
        config: Config,
        ranker: R,
        stacks_ops: Vec<BoxedOps>,
    ) -> Result<Self, Error> {
        if stacks_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let mut stacks_data = Self::deserialize(state)?;

        let stacks = stacks_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                let data = stacks_data.remove(&id).unwrap_or_default();

                Stack::new(data, ops).map(|stack| (id, RwLock::new(stack)))
            })
            .collect::<Result<_, _>>()
            .map_err(Error::InvalidStack)?;

        Ok(Self {
            config,
            stacks,
            ranker,
        })
    }

    /// Serializes the state of the `Engine`.
    ///
    /// The result can be used with [`Engine::new`] to restore it.
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let state = self
            .stacks
            .iter()
            .map(|(&id, stack)| async move {
                bincode::serialize(&stack.read().await.data)
                    .map(|data| (id, data))
                    .map_err(Error::Serialization)
            })
            .collect::<FuturesUnordered<_>>()
            .try_collect::<HashMap<_, _>>()
            .await?;

        bincode::serialize(&state).map_err(Error::Serialization)
    }

    /// Deserializes the state of the engine.
    fn deserialize(state: &[u8]) -> Result<HashMap<StackId, StackData>, Error> {
        bincode::deserialize::<HashMap<_, Vec<_>>>(state)
            .map_err(Error::Deserialization)?
            .into_iter()
            .map(|(id, data)| {
                bincode::deserialize(&data)
                    .map(|data| (id, data))
                    .map_err(Error::Deserialization)
            })
            .collect()
    }

    /// Returns at most `max_documents` [`Document`]s for the feed.
    pub async fn get_feed_documents(&self, max_documents: usize) -> Result<Vec<Document>, Error> {
        Selection::new(BetaSampler, self.stacks.values())
            .select(max_documents)
            .await
            .map_err(|e| e.into())
    }

    /// The ranker could rank the documents in a different order so we update the stacks with it.
    async fn rank_stacks(&self) -> Result<(), Error> {
        let errors = self
            .stacks
            .values()
            .into_iter()
            .map(|stack| async move { stack })
            .collect::<FuturesUnordered<_>>()
            .fold(vec![], |mut errors, stack| async move {
                if let Err(error) = stack.write().await.rank(&mut self.ranker) {
                    errors.push(Error::StackOpFailed(error));
                }

                errors
            })
            .await;

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Errors(errors))
        }
    }

    /// Process the feedback about the user spending some time on a document.
    pub async fn time_spent(&mut self, time_spent: &TimeSpent) -> Result<(), Error> {
        self.ranker.log_document_view_time(time_spent)?;

        self.rank_stacks().await
    }

    /// Process the feedback about the user reacting to a document.
    pub async fn user_reacted(&mut self, reacted: &UserReacted) -> Result<(), Error> {
        self.stacks
            .get(&reacted.stack_id)
            .ok_or(Error::InvalidStackId(reacted.stack_id))?
            .write()
            .await
            .update_relevance(reacted.reaction);

        self.ranker.log_user_reaction(reacted)?;

        self.rank_stacks().await
    }

    /// Updates the stacks with data related to the top key phrases of the current data.
    #[allow(dead_code)]
    async fn update_stacks(&mut self, top: usize) -> Result<(), Error> {
        let key_phrases = self.ranker.select_top_key_phrases(top);
        let key_phrases = &key_phrases;
        let ranker = &mut self.ranker;

        self.stacks
            .values()
            .map(|stack| async move {
                stack
                    .read()
                    .await
                    .ops
                    .new_items(key_phrases, ranker)
                    .await
                    .map(|documents| (stack, documents))
            })
            .collect::<FuturesUnordered<_>>()
            .try_for_each(|(stack, documents)| async move {
                stack
                    .write()
                    .await
                    .update(&documents, ranker)
                    .map_err(GenericError::from)
            })
            .await
            .map_err(Into::into)
    }
}

/// A wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;
