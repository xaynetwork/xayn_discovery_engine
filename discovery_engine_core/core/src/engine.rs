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
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use xayn_ai::{
    ranker::{AveragePooler, Builder},
    KpeConfig,
    SMBertConfig,
};

use crate::{
    document::{Document, TimeSpent, UserReacted},
    mab::{self, BetaSampler, SelectionIter},
    ranker::Ranker,
    stack::{self, BoxedOps, Data as StackData, Id as StackId, Stack},
};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to serialize internal state of the engine: {0}.
    Serialization(#[source] GenericError),

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

/// Discovery Engine configuration settings.
#[allow(dead_code)]
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

/// Temporary config to allow for configurations within the core without a mirroring outside impl.
struct CoreConfig {
    /// The number of selected top key phrases while updating the stacks.
    select_top: usize,
    /// The number of top documents per stack to keep while filtering the stacks.
    keep_top: usize,
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            select_top: 3,
            keep_top: 20,
        }
    }
}

/// Discovery Engine.
pub struct Engine<R> {
    #[allow(dead_code)]
    config: Config,
    core_config: CoreConfig,
    stacks: RwLock<HashMap<StackId, Stack>>,
    ranker: R,
}

impl<R> Engine<R>
where
    R: Ranker + Send + Sync,
{
    /// Creates a new `Engine` from configuration.
    pub fn new(config: Config, ranker: R, stack_ops: Vec<BoxedOps>) -> Result<Self, Error> {
        let stack_data = |_| StackData::default();

        Self::from_stack_data(config, ranker, stack_data, stack_ops)
    }

    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    pub fn from_state(
        state: &StackState,
        config: Config,
        ranker: R,
        stack_ops: Vec<BoxedOps>,
    ) -> Result<Self, Error> {
        if stack_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let mut stack_data = bincode::deserialize::<HashMap<StackId, _>>(&state.0)
            .map_err(Error::Deserialization)?;
        let stack_data = |id| stack_data.remove(&id).unwrap_or_default();

        Self::from_stack_data(config, ranker, stack_data, stack_ops)
    }

    fn from_stack_data(
        config: Config,
        ranker: R,
        mut stack_data: impl FnMut(StackId) -> StackData,
        stack_ops: Vec<BoxedOps>,
    ) -> Result<Self, Error> {
        let stacks = stack_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                let data = stack_data(id);
                Stack::new(data, ops).map(|stack| (id, stack))
            })
            .collect::<Result<_, _>>()
            .map(RwLock::new)
            .map_err(Error::InvalidStack)?;
        let core_config = CoreConfig::default();

        Ok(Self {
            config,
            core_config,
            stacks,
            ranker,
        })
    }

    /// Serializes the state of the `Engine` and `Ranker` state.
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();

        let engine = bincode::serialize(&stacks_data)
            .map(StackState)
            .map_err(|err| Error::Serialization(err.into()))?;

        let ranker = self
            .ranker
            .serialize()
            .map(RankerState)
            .map_err(Error::Serialization)?;

        let state_data = State { engine, ranker };

        bincode::serialize(&state_data).map_err(|err| Error::Serialization(err.into()))
    }

    /// Returns at most `max_documents` [`Document`]s for the feed.
    pub async fn get_feed_documents(&self, max_documents: usize) -> Result<Vec<Document>, Error> {
        SelectionIter::new(BetaSampler, self.stacks.write().await.values_mut())
            .select(max_documents)
            .map_err(Into::into)
    }

    /// Process the feedback about the user spending some time on a document.
    pub async fn time_spent(&mut self, time_spent: &TimeSpent) -> Result<(), Error> {
        self.ranker.log_document_view_time(time_spent)?;

        rank_stacks(self.stacks.write().await.values_mut(), &mut self.ranker)
    }

    /// Process the feedback about the user reacting to a document.
    pub async fn user_reacted(&mut self, reacted: &UserReacted) -> Result<(), Error> {
        let mut stacks = self.stacks.write().await;
        stacks
            .get_mut(&reacted.stack_id)
            .ok_or(Error::InvalidStackId(reacted.stack_id))?
            .update_relevance(reacted.reaction);

        self.ranker.log_user_reaction(reacted)?;

        rank_stacks(stacks.values_mut(), &mut self.ranker)
    }

    /// Updates the stacks with data related to the top key phrases of the current data.
    #[allow(dead_code)]
    async fn update_stacks(&mut self) -> Result<(), Error> {
        let key_phrases = &self
            .ranker
            .select_top_key_phrases(self.core_config.select_top);

        let mut errors = Vec::new();
        for stack in self.stacks.write().await.values_mut() {
            match stack.ops.new_items(key_phrases, &self.ranker).await {
                Ok(documents) => {
                    if let Err(error) = stack.update(&documents, &mut self.ranker) {
                        errors.push(Error::StackOpFailed(error));
                    }
                }
                Err(error) => errors.push(error.into()),
            }
            stack.data.retain_top(self.core_config.keep_top);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Errors(errors))
        }
    }
}

/// The ranker could rank the documents in a different order so we update the stacks with it.
fn rank_stacks<'a>(
    stacks: impl Iterator<Item = &'a mut Stack>,
    ranker: &mut impl Ranker,
) -> Result<(), Error> {
    let errors = stacks.fold(Vec::new(), |mut errors, stack| {
        if let Err(error) = stack.rank(ranker) {
            errors.push(Error::StackOpFailed(error));
        }

        errors
    });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Errors(errors))
    }
}

impl Engine<xayn_ai::ranker::Ranker> {
    /// Creates a discovery engine with [`xayn_ai::ranker::Ranker`] as a ranker.
    pub fn from_config(
        config: Config,
        stacks_ops: Vec<BoxedOps>,
        state: Option<&[u8]>,
    ) -> Result<Engine<impl Ranker>, Error> {
        let smbert_config = SMBertConfig::from_files(&config.smbert_vocab, &config.smbert_model)
            .map_err(|err| Error::Ranker(err.into()))?
            .with_token_size(52)
            .map_err(|err| Error::Ranker(err.into()))?
            .with_accents(false)
            .with_lowercase(true)
            .with_pooling(AveragePooler);

        let kpe_config = KpeConfig::from_files(
            &config.kpe_vocab,
            &config.kpe_model,
            &config.kpe_cnn,
            &config.kpe_classifier,
        )
        .map_err(|err| Error::Ranker(err.into()))?
        .with_token_size(150)
        .map_err(|err| Error::Ranker(err.into()))?
        .with_accents(false)
        .with_lowercase(false);

        let builder = Builder::from(smbert_config, kpe_config);

        if let Some(state) = state {
            let state: State = bincode::deserialize(state).map_err(Error::Deserialization)?;
            let ranker = builder
                .with_serialized_state(&state.ranker.0)
                .map_err(|err| Error::Ranker(err.into()))?
                .build()
                .map_err(|err| Error::Ranker(err.into()))?;
            Engine::from_state(&state.engine, config, ranker, stacks_ops)
        } else {
            let ranker = builder.build().map_err(|err| Error::Ranker(err.into()))?;
            Engine::new(config, ranker, stacks_ops)
        }
    }
}

/// A wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

#[derive(Serialize, Deserialize)]
pub struct StackState(Vec<u8>);

#[derive(Serialize, Deserialize)]
struct RankerState(Vec<u8>);

#[derive(Serialize, Deserialize)]
struct State {
    /// The serialized engine state.
    engine: StackState,
    /// The serialized ranker state.
    ranker: RankerState,
}
