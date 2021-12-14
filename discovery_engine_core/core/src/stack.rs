//! Export types to customize the behaviour of a stack.

use derive_more::From;
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    document::Document,
    engine::{GenericError, Ranker},
    mab::Bucket,
};

mod data;
mod ops;

pub(crate) use data::Data;
pub use ops::Ops;

#[derive(Error, Debug, Display)]
#[allow(dead_code)]
pub(crate) enum Error {
    /// Failed to merge current documents with new ones.
    Merge(#[source] GenericError),
    /// Failed to rank documents when updating the stack: {0}.
    Ranking(#[source] GenericError),
}

/// Convenience type that boxes an [`ops::Ops`] and adds [`Send`] and [`Sync`].
pub type BoxStackOps = Box<dyn Ops + Send + Sync>;

/// Id of a stack.
///
/// `Id` is used to connect [`Ops`](ops::Ops) with the corresponding data of a stack.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, From)]
pub struct Id(Uuid);

pub(crate) struct Stack {
    pub(crate) data: Data,
    pub(crate) ops: BoxStackOps,
}

impl Stack {
    /// Create a new `Stack` with the given [`Data`] and customized [`Ops`].
    #[allow(dead_code)]
    pub(crate) fn new(data: Data, ops: BoxStackOps) -> Self {
        Self { data, ops }
    }

    /// [`Id`] of this `Stack`.
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> Id {
        self.ops.id()
    }

    /// Ranks the slice of [`Document`] items and returns an updated [`Stack`].
    #[allow(dead_code)]
    pub(crate) fn update<R: Ranker>(
        mut self,
        new_documents: &[Document],
        ranker: &R,
    ) -> Result<Self, Error> {
        let mut items = self
            .ops
            .merge(&self.data.documents, new_documents)
            .map_err(Error::Merge)?;
        ranker.rank(&mut items).map_err(Error::Ranking)?;
        self.data.documents = items;
        Ok(self)
    }
}

impl Bucket<Document> for Stack {
    fn alpha(&self) -> f32 {
        self.data.alpha
    }

    fn beta(&self) -> f32 {
        self.data.beta
    }

    fn is_empty(&self) -> bool {
        self.data.documents.is_empty()
    }

    fn pop(&mut self) -> Option<Document> {
        self.data.documents.pop()
    }
}

#[cfg(test)]
mod tests {
    use claim::{assert_none, assert_some};
    use ndarray::arr1;

    use crate::{document::Embedding, stack::ops::MockOps, Id};

    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_bucket_pop_empty() {
        let mut stack = Stack::new(Data::default(), Box::new(MockOps::new()));

        assert_none!(stack.pop());
    }

    #[test]
    fn test_stack_bucket_pop() {
        let doc = Document {
            id: Id::from_u128(u128::MIN),
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[1., 2., 3.])),
        };
        let data = Data::new(0.01, 0.99, vec![doc.clone(), doc]).unwrap();
        let mut stack = Stack::new(data, Box::new(MockOps::new()));

        assert_some!(stack.pop());
        assert_some!(stack.pop());
        assert_none!(stack.pop());
    }
}
