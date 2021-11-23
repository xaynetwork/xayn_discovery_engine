use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::document::Document;

#[derive(Error, Debug, Display)]
pub enum DiscoveryEngineError {
    /// failed to serialize internal state of the engine
    Serialization(bincode::Error),
    /// failed to deserialze internal state to create the engine
    Deserialization(bincode::Error),
}

/// DiscoveryEngine
pub struct DiscoveryEngine {
    /// Internal state of the Discovery Engine
    state: InternalState,
}

impl DiscoveryEngine {
    /// Creates a new [`DiscoveryEngine`] from serialized state.
    pub fn new(state: &[u8]) -> Result<Self, DiscoveryEngineError> {
        let state = bincode::deserialize(state).map_err(DiscoveryEngineError::Deserialization)?;
        Ok(DiscoveryEngine { state })
    }

    /// Serializes [`InternalState`] of the engine.
    pub fn serialize(&self) -> Result<Vec<u8>, DiscoveryEngineError> {
        bincode::serialize(&self.state).map_err(DiscoveryEngineError::Serialization)
    }
}

/// Internal state of Discovery Engine
#[derive(Deserialize, Serialize)]
pub(crate) struct InternalState {
    /// Stack of news in a news feed
    pub(crate) news_feed: Stack,
    /// Stack of personalized news
    pub(crate) personalized_news: Stack,
}

/// TODO: add documentation
#[derive(Deserialize, Serialize)]
pub(crate) struct Stack {
    /// TODO: add documentation
    pub(crate) alpha: f32,
    /// TODO: add documentation
    pub(crate) beta: f32,
    /// TODO: add documentation
    pub(crate) documents: Vec<Document>,
}

impl Stack {
    /// Creates a new Stack.
    pub(crate) fn new(alpha: f32, beta: f32, documents: Vec<Document>) -> Self {
        Self {
            alpha,
            beta,
            documents,
        }
    }
}
