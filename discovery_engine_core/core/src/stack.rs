use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    document::Document,
    engine::{GenericError, Ranker},
};

#[derive(Error, Debug, Display)]
pub(crate) enum Error {
    /// invalid value for alpha: {0}
    InvalidAlpha(f32),
    /// invalid value for beta: {0}
    InvalidBeta(f32),
    /// failed to rank documents when updating the stack: {0}
    Ranking(#[source] GenericError),
}

/// Stack of feed items
#[derive(Deserialize, Serialize)]
pub(crate) struct Stack {
    /// The alpha parameter of the beta distribution
    pub(crate) alpha: f32,
    /// The beta parameter of the beta distribution
    pub(crate) beta: f32,
    /// Documents in the [`Stack`]
    pub(crate) documents: Vec<Document>,
}

impl Stack {
    /// Creates a new Stack.
    pub(crate) fn new(alpha: f32, beta: f32, documents: Vec<Document>) -> Result<Self, Error> {
        if alpha <= 0.0 || alpha > 1.0 {
            return Err(Error::InvalidAlpha(alpha));
        }
        if beta <= 0.0 || beta > 1.0 {
            return Err(Error::InvalidBeta(beta));
        }

        Ok(Self {
            alpha,
            beta,
            documents,
        })
    }

    /// Ranks the slice of [`Document`] items and returns an updated [`Stack`]
    pub(crate) fn _update<R: Ranker>(
        mut self,
        new_feed_items: &[Document],
        ranker: &R,
    ) -> Result<Self, Error> {
        self.documents.extend_from_slice(new_feed_items);
        ranker.rank(&mut self.documents).map_err(Error::Ranking)?;

        Ok(self)
    }
}
