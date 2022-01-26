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
use thiserror::Error;

use crate::{
    document::{Document, TimeSpent, UserReacted},
    mab::{self, BetaSampler, Selection},
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

/// Discovery Engine.
pub struct Engine<R> {
    stacks: HashMap<StackId, Stack>,
    ranker: R,
}

impl<R> Engine<R>
where
    R: Ranker,
{
    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    pub fn new(state: &[u8], ranker: R, stacks_ops: Vec<BoxedOps>) -> Result<Self, Error> {
        if stacks_ops.is_empty() {
            return Err(Error::NoStackOps);
        }

        let mut stacks_data: HashMap<StackId, StackData> =
            bincode::deserialize(state).map_err(Error::Deserialization)?;

        let stacks = stacks_ops
            .into_iter()
            .map(|ops| {
                let id = ops.id();
                let data = stacks_data.remove(&id).unwrap_or_default();

                Stack::new(data, ops).map(|stack| (id, stack))
            })
            .collect::<Result<_, _>>()
            .map_err(Error::InvalidStack)?;

        Ok(Engine { stacks, ranker })
    }

    /// Serializes the state of the `Engine`.
    ///
    /// The result can be used with [`Engine::new`] to restore it.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks_data: HashMap<&StackId, &StackData> = self
            .stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect();

        bincode::serialize(&stacks_data).map_err(Error::Serialization)
    }

    /// Returns at most `max_documents` [`Document`]s for the feed.
    pub async fn get_feed_documents(&mut self, max_documents: u32) -> Result<Vec<Document>, Error>
    where
        R: Send + Sync,
    {
        Selection::new(BetaSampler)
            .select(self.stacks.values_mut().collect(), max_documents)
            .map_err(|e| e.into())
    }

    /// Process the feedback about the user spending some time on a document.
    pub fn time_logged(&mut self, time_logged: &TimeSpent) -> Result<(), Error> {
        self.ranker.time_logged(time_logged)?;

        rank_stacks(self.stacks.values_mut(), &self.ranker)
    }

    /// Process the feedback about the user reacting to a document.
    pub fn user_reacted(&mut self, reacted: &UserReacted) -> Result<(), Error> {
        let stack = self
            .stacks
            .get_mut(&reacted.stack_id)
            .ok_or(Error::InvalidStackId(reacted.stack_id))?;

        stack.update_relevance(reacted.reaction);

        self.ranker.user_reacted(reacted)?;

        rank_stacks(self.stacks.values_mut(), &self.ranker)
    }
}

/// The ranker could rank the documents in a different order so we update the stacks with it.
fn rank_stacks<'a, R: Ranker>(
    stacks: impl Iterator<Item = &'a mut Stack>,
    ranker: &R,
) -> Result<(), Error> {
    let errors = stacks.fold(vec![], |mut errors, stack| {
        if let Err(e) = stack.rank(ranker).map_err(Error::StackOpFailed) {
            errors.push(e);
        }

        errors
    });

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Error::Errors(errors))
    }
}

/// A wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

/// Provides a method for ranking a slice of [`Document`] items.
pub trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&self, items: &mut [Document]) -> Result<(), GenericError>;

    /// Learn from the time a user spent on a document.
    fn time_logged(&mut self, time_logged: &TimeSpent) -> Result<(), GenericError>;

    /// Learn from a user's interaction.
    fn user_reacted(&mut self, reaction: &UserReacted) -> Result<(), GenericError>;
}
