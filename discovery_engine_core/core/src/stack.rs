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
    /// Invalid value for alpha: {0}. It must be greater than 0.
    InvalidAlpha(f32),
    /// Invalid value for beta: {0}. It must be greater than 0.
    InvalidBeta(f32),
    /// Failed to rank documents when updating the stack: {0}.
    Ranking(#[source] GenericError),
}

/// Stack of feed items
#[derive(Deserialize, Serialize, Debug)]
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
        if alpha <= 0.0 {
            return Err(Error::InvalidAlpha(alpha));
        }
        if beta <= 0.0 {
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
    use claim::{assert_err, assert_matches, assert_none, assert_ok, assert_some};
    use ndarray::arr1;

    use crate::{document::Embedding, Id};

    use super::*;

    #[test]
    fn test_stack_initialisation() {
        let stack_0 = Stack::new(0. + f32::EPSILON, 0. + f32::EPSILON, vec![]);
        let stack_1 = Stack::new(0.0, 0.5, vec![]);
        let stack_2 = Stack::new(0.5, 0.0, vec![]);
        let stack_3 = Stack::new(-0.0, 1.0, vec![]);
        let stack_4 = Stack::new(1.0, -0.0, vec![]);
        let stack_5 = Stack::new(-1.0, 1.0, vec![]);
        let stack_6 = Stack::new(1.0, -1.0, vec![]);

        assert_ok!(stack_0);
        assert_err!(&stack_1);
        assert_matches!(stack_1.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);
        assert_err!(&stack_2);
        assert_matches!(stack_2.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);
        assert_err!(&stack_3);
        assert_matches!(stack_3.unwrap_err(), Error::InvalidAlpha(x) if x == 0.0);
        assert_err!(&stack_4);
        assert_matches!(stack_4.unwrap_err(), Error::InvalidBeta(x) if x == 0.0);
        assert_err!(&stack_5);
        assert_matches!(stack_5.unwrap_err(), Error::InvalidAlpha(x) if x == -1.0);
        assert_err!(&stack_6);
        assert_matches!(stack_6.unwrap_err(), Error::InvalidBeta(x) if x == -1.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_bucket_impl() {
        let mut stack_0 = Stack::new(0.01, 0.99, vec![]).unwrap();

        assert_eq!(stack_0.alpha(), 0.01);
        assert_eq!(stack_0.beta(), 0.99);
        assert!(stack_0.is_empty());
        assert_none!(stack_0.pop());

        let doc_1 = Document {
            id: Id::from_u128(u128::MIN),
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[1., 2., 3.])),
        };
        let mut stack_1 = Stack::new(0.01, 0.99, vec![doc_1]).unwrap();

        assert!(!stack_1.is_empty());
        assert_some!(stack_1.pop());
        assert!(stack_1.is_empty());
    }
}
