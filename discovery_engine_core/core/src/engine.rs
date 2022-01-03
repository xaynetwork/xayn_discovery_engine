use std::collections::HashMap;

use displaydoc::Display;
use thiserror::Error;

use crate::{
    document::Document,
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

    /// Error while selecting the document to return: {0}.
    Selection(#[from] mab::Error),
}

/// Discovery Engine.
pub struct Engine {
    stacks: HashMap<StackId, Stack>,
}

impl Engine {
    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keeps in its state data related to the current [`BoxedOps`].
    /// Data related to missing operations will be dropped.
    pub fn new(state: &[u8], stacks_ops: Vec<BoxedOps>) -> Result<Self, Error> {
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

        Ok(Engine { stacks })
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
    pub fn get_feed_documents(&mut self, max_documents: u32) -> Result<Vec<Document>, Error> {
        Selection::new(BetaSampler)
            .select(self.stacks.values_mut().collect(), max_documents)
            .map_err(|e| e.into())
    }
}

/// A wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

/// Provides a method for ranking slice of [`Document`] items.
pub(crate) trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&self, items: &mut [Document]) -> Result<(), GenericError>;
}
