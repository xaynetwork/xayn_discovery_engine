use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    document::Document,
    engine::{GenericError, Ranker},
    mab::Bucket,
};

#[derive(Error, Debug, Display)]
#[allow(dead_code)]
pub(crate) enum Error {
    /// Invalid value for alpha: {0}. It must be in range [0, 1].
    InvalidAlpha(f32),
    /// Invalid value for beta: {0}. It must be in range [0, 1].
    InvalidBeta(f32),
    /// Failed to rank documents when updating the stack: {0}.
    Ranking(#[source] GenericError),
}

/// Stack of feed items
#[derive(Deserialize, Serialize)]
pub(crate) struct Stack {
    /// The alpha parameter of the beta distribution.
    alpha: f32,
    /// The beta parameter of the beta distribution.
    beta: f32,
    /// Documents in the [`Stack`].
    pub(crate) documents: Vec<Document>,
}

impl Stack {
    #[allow(dead_code)]
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

    /// Ranks the slice of [`Document`] items and returns an updated [`Stack`].
    #[allow(dead_code)]
    pub(crate) fn update<R: Ranker>(
        mut self,
        new_feed_items: &[Document],
        ranker: &R,
    ) -> Result<Self, Error> {
        self.documents.extend_from_slice(new_feed_items);
        ranker.rank(&mut self.documents).map_err(Error::Ranking)?;

        Ok(self)
    }
}

impl Bucket<Document> for Stack {
    fn alpha(&self) -> f32 {
        self.alpha
    }

    fn beta(&self) -> f32 {
        self.beta
    }

    fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    fn pop(&mut self) -> Option<Document> {
        self.documents.pop()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let mut stack_0 = Stack {
            alpha: 0.01,
            beta: 1.0,
            documents: vec![],
        };

        let _stacks = vec![&mut stack_0];
        // let mab = Selection::new(BetaSampler);

        // let docs = mab.select(stacks, 10).unwrap();
        // assert_eq!(docs[0], 3);
        // assert_eq!(docs[1], 2);
        // assert_eq!(docs[2], 1);
        // assert_eq!(docs[3], 0);
        // assert_eq!(docs[4], 6);
        // assert_eq!(docs[5], 5);
        // assert_eq!(docs[6], 4);
    }
}
