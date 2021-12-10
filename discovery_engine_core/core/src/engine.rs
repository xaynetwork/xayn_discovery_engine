use std::collections::HashMap;

use displaydoc::Display;
use thiserror::Error;

use crate::{
    document::Document,
    stack::{BoxStackOps, Data as StackData, Id as StackId, Stack},
};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to serialize internal state of the engine: {0}.
    Serialization(#[source] bincode::Error),

    /// Failed to deserialize internal state to create the engine: {0}.
    Deserialization(#[source] bincode::Error),

    /// No operations on stack were provided.
    NoStackOps,
}

/// Discovery Engine.
pub struct Engine {
    stacks: HashMap<StackId, Stack>,
}

impl Engine {
    /// Creates a new `Engine` from serialized state and stack operations.
    ///
    /// The `Engine` only keep in its state data related to the current [`BoxStackOps`],
    /// data related to missing operations will be dropped.
    pub fn new(state: &[u8], stacks_ops: Vec<BoxStackOps>) -> Result<Self, Error> {
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
                (id, Stack::new(data, ops))
            })
            .collect();

        Ok(Engine { stacks })
    }

    /// Serializes [`InternalState`] of the engine.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks_data: HashMap<&StackId, &StackData> = self
            .stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect();

        bincode::serialize(&stacks_data).map_err(Error::Serialization)
    }
}

/// A a wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

/// Provides a method for ranking slice of [`Document`] items.
pub(crate) trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&self, items: &mut [Document]) -> Result<(), GenericError>;
}
