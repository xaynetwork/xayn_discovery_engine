use crate::document::Document;

/// DiscoveryEngine
pub struct DiscoveryEngine {
    /// Internal state of the Discovery Engine
    pub state: InternalState,
}

impl DiscoveryEngine {
    /// TODO: add documentation
    pub fn new(state: InternalState) -> Self {
        Self { state }
    }

    /// TODO: add documentation
    pub fn serialize(&self) -> InternalState {
        self.state.clone()
    }
}

/// Internal state of Discovery Engine
#[derive(Clone)]
pub struct InternalState {
    /// Stack of news in a news feed
    pub news_feed: Stack,
    /// Stack of personalized news
    pub personalized_news: Stack,
}

/// TODO: add documentation
#[derive(Clone)]
pub struct Stack {
    /// TODO: add documentation
    pub alpha: f32,
    /// TODO: add documentation
    pub beta: f32,
    /// TODO: add documentation
    pub document: Vec<Document>,
}

impl Stack {
    /// TODO: add documentation
    pub fn new(alpha: f32, beta: f32, document: Vec<Document>) -> Self {
        Self {
            alpha,
            beta,
            document,
        }
    }
}
