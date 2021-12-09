use derive_more::From;
use displaydoc::Display;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    document::Document,
    engine::{GenericError, Ranker},
    mab::Bucket,
};

mod data;
mod ops;
// mod stacks;

pub(crate) use data::StackData;
pub use ops::Ops;

#[derive(Error, Debug, Display)]
#[allow(dead_code)]
pub(crate) enum Error {
    /// Failed to merge current documents with new ones.
    Merge(#[source] GenericError),
    /// Failed to rank documents when updating the stack: {0}.
    Ranking(#[source] GenericError),
}

pub type BoxOps = Box<dyn Ops + Send + Sync>;

/// Id of a [`Stack`].
///
/// `Id` is used to connect [`ops::Ops`] with [`data::Data`].
#[derive(From)]
pub struct Id(Uuid);

pub(crate) struct Stack {
    pub(crate) data: StackData,
    pub(crate) ops: BoxOps,
}

impl Stack {
    /// Create a new `Stack` with the given [`StackData`] and customized [`Ops`].
    #[allow(dead_code)]
    pub(crate) fn new(data: StackData, ops: BoxOps) -> Self {
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
        new_feed_items: &[Document],
        ranker: &R,
    ) -> Result<Self, Error> {
        let mut items = self
            .ops
            .merge(&self.data.documents, new_feed_items)
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
        let mut stack = Stack::new(StackData::empty(), Box::new(MockOps::new()));

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
        let data = StackData::from_parts(0.01, 0.99, vec![doc.clone(), doc]).unwrap();
        let mut stack = Stack::new(data, Box::new(MockOps::new()));

        assert_some!(stack.pop());
        assert_some!(stack.pop());
        assert_none!(stack.pop());
    }
}
