use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::document::Document;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// failed to serialize internal state of the engine: {0}
    Serialization(#[source] bincode::Error),
    /// failed to deserialze internal state to create the engine: {0}
    Deserialization(#[source] bincode::Error),
}

/// Discovery Engine
pub struct Engine {
    /// Internal state of the engine
    state: InternalState,
}

impl Engine {
    /// Creates a new [`Engine`] from serialized state.
    pub fn new(state: &[u8]) -> Result<Self, Error> {
        let state = bincode::deserialize(state).map_err(Error::Deserialization)?;
        Ok(Engine { state })
    }

    /// Serializes [`InternalState`] of the engine.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self.state).map_err(Error::Serialization)
    }
}

/// Internal state of [`Engine`]
#[derive(Deserialize, Serialize)]
pub(crate) struct InternalState {
    /// Stack of news in a news feed
    pub(crate) news_feed: Stack,
    /// Stack of personalized news
    pub(crate) personalized_news: Stack,
}

/// Stack of feed items
#[derive(Deserialize, Serialize)]
pub(crate) struct Stack {
    /// TODO: add documentation
    pub(crate) alpha: f32,
    /// TODO: add documentation
    pub(crate) beta: f32,
    /// Documents in the [`Stack`]
    pub(crate) documents: Vec<Document>,
}

impl Stack {
    /// Creates a new Stack.
    pub(crate) fn _new(alpha: f32, beta: f32, documents: Vec<Document>) -> Self {
        Self {
            alpha,
            beta,
            documents,
        }
    }
}
