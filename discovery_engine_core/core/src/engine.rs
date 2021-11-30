use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{document::Document, stack::Stack};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Failed to serialize internal state of the engine: {0}.
    Serialization(#[source] bincode::Error),
    /// Failed to deserialize internal state to create the engine: {0}.
    Deserialization(#[source] bincode::Error),
}

/// Discovery Engine.
pub struct Engine {
    /// Internal state of the engine.
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

/// Internal state of [`Engine`].
#[derive(Deserialize, Serialize)]
pub(crate) struct InternalState {
    /// Stack of news in a news feed.
    pub(crate) news_feed: Stack,
    /// Stack of personalized news.
    pub(crate) personalized_news: Stack,
}

/// A a wrapper around a dynamic error type, similar to `anyhow::Error`,
/// but without the need to declare `anyhow` as a dependency.
pub(crate) type GenericError = Box<dyn std::error::Error + Sync + Send + 'static>;

/// Provides a method for ranking slice of [`Document`] items.
pub(crate) trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&self, items: &mut [Document]) -> Result<(), GenericError>;
}
