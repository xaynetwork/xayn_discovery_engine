//! Export types to customize the behaviour of a stack.

use derivative::Derivative;
use derive_more::{Display, From};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    document::{Document, Id as DocumentId},
    engine::{GenericError, Ranker},
    mab::Bucket,
};

mod data;
mod ops;

pub(crate) use data::Data;
pub use ops::Ops;

/// Errors that could occurs while manipulating a stack.
#[derive(Error, Debug, DisplayDoc)]
#[allow(dead_code)]
pub enum Error {
    /// Failed to merge current documents with new ones.
    Merge(#[source] GenericError),

    /// Failed to rank documents when updating the stack: {0}.
    Ranking(#[source] GenericError),

    /// [`Document`] {document_id} has stack id {document_stack_id} instead of {stack_id}.
    InvalidDocument {
        /// [`DocumentId`] of the offending document.
        document_id: DocumentId,
        /// [`StackId`](Id) of the document.
        document_stack_id: Id,
        /// [`StackId`](Id) of the the current stack.
        stack_id: Id,
    },
}

/// Convenience type that boxes an [`ops::Ops`] and adds [`Send`] and [`Sync`].
pub type BoxedOps = Box<dyn Ops + Send + Sync>;

/// Id of a stack.
///
/// `Id` is used to connect [`Ops`](ops::Ops) with the corresponding data of a stack.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, From, Display)]
pub struct Id(Uuid);

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Stack {
    pub(crate) data: Data,
    #[derivative(Debug = "ignore")]
    pub(crate) ops: BoxedOps,
}

impl Stack {
    /// Create a new `Stack` with the given [`Data`] and customized [`Ops`].
    #[allow(dead_code)]
    pub(crate) fn new(data: Data, ops: BoxedOps) -> Result<Self, Error> {
        Self::validate_documents_stack_id(&data.documents, ops.id())?;

        Ok(Self { data, ops })
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
        Self::validate_documents_stack_id(new_documents, self.ops.id())?;

        let mut items = self
            .ops
            .merge(&self.data.documents, new_documents)
            .map_err(Error::Merge)?;
        ranker.rank(&mut items).map_err(Error::Ranking)?;
        self.data.documents = items;
        Ok(self)
    }

    /// It checks that every document belongs to a stack.
    fn validate_documents_stack_id(documents: &[Document], stack_id: Id) -> Result<(), Error> {
        if let Some(doc) = documents.iter().find(|doc| doc.stack_id != stack_id) {
            return Err(Error::InvalidDocument {
                document_id: doc.id,
                document_stack_id: doc.stack_id,
                stack_id,
            });
        }

        Ok(())
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
    use claim::{assert_matches, assert_none, assert_ok, assert_some};
    use ndarray::arr1;
    use uuid::Uuid;

    use std::fmt::Debug;

    use crate::{
        document::{Embedding, Id as DocumentId},
        stack::ops::MockOps,
    };

    use super::*;

    // checks that `f` returns ok if the argument contains only documents valid `stack_id`
    fn check_valid_document<T, F>(f: F, stack_id: Id)
    where
        T: Debug,
        F: Fn(&[Document]) -> Result<T, Error>,
    {
        let doc_1 = Document {
            id: DocumentId::from_u128(u128::MIN),
            stack_id,
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        let doc_2 = Document {
            id: DocumentId::from_u128(u128::MAX),
            stack_id,
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        assert_ok!(f(&[]));
        assert_ok!(f(&[doc_1.clone()]));
        assert_ok!(f(&[doc_1, doc_2]));
    }

    // checks that `f` returns an error if the argument contains a Document with an invalid `stack_id`
    fn check_invalid_document<T, F>(f: F, stack_id_ok: Id)
    where
        T: Debug,
        F: Fn(&[Document]) -> Result<T, Error>,
    {
        let stack_id_ko = Uuid::from_u128(stack_id_ok.0.as_u128() + 1).into();
        let doc_id_ko = DocumentId::from_u128(u128::MAX);

        let doc_ok = Document {
            id: DocumentId::from_u128(u128::MIN),
            stack_id: stack_id_ok,
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        let doc_ko = Document {
            id: doc_id_ko,
            stack_id: stack_id_ko,
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        let assert_invalid_document = |docs: &[Document]| {
            assert_matches!(
                f(docs),
                Err(Error::InvalidDocument { document_id, document_stack_id, stack_id})
                    if document_id == doc_ko.id && document_stack_id == doc_ko.stack_id && stack_id == stack_id_ok);
        };

        assert_invalid_document(&[doc_ko.clone()]);
        assert_invalid_document(&[doc_ko.clone(), doc_ok.clone()]);
        assert_invalid_document(&[doc_ok.clone(), doc_ko.clone()]);
        assert_invalid_document(&[doc_ok.clone(), doc_ok.clone(), doc_ko.clone()]);
        assert_invalid_document(&[doc_ok.clone(), doc_ko.clone(), doc_ok]);
    }

    #[test]
    fn test_stack_validate_documents_stack_id_ok() {
        let stack_id = Uuid::from_u128(1).into();

        check_valid_document(
            |docs| Stack::validate_documents_stack_id(docs, stack_id),
            stack_id,
        );
    }

    #[test]
    fn test_stack_validate_documents_stack_id_ko() {
        let stack_id = Uuid::from_u128(1).into();

        check_invalid_document(
            |docs| Stack::validate_documents_stack_id(docs, stack_id),
            stack_id,
        );
    }

    #[test]
    fn test_stack_new_from_default() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(|| Uuid::from_u128(1).into());
        assert_ok!(Stack::new(Data::default(), Box::new(ops)));
    }

    #[test]
    fn test_stack_new_valid_documents() {
        let stack_id = Uuid::from_u128(1).into();

        check_valid_document(
            |docs| {
                let mut ops = MockOps::new();
                ops.expect_id().returning(move || stack_id);

                let data = Data::new(0.01, 0.99, docs.to_vec()).unwrap();
                Stack::new(data, Box::new(ops))
            },
            stack_id,
        );
    }

    #[test]
    fn test_stack_new_invalid_documents() {
        let stack_id = Uuid::from_u128(1).into();

        check_invalid_document(
            |docs| {
                let mut ops = MockOps::new();
                ops.expect_id().returning(move || stack_id);

                let data = Data::new(0.01, 0.99, docs.to_vec()).unwrap();
                Stack::new(data, Box::new(ops))
            },
            stack_id,
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_stack_bucket_alpha_beta() {
        let alpha = 0.01;
        let beta = 0.99;

        let mut ops = MockOps::new();
        ops.expect_id().returning(move || Uuid::nil().into());

        let data = Data::new(alpha, beta, vec![]).unwrap();
        let stack = Stack::new(data, Box::new(ops)).unwrap();

        assert_eq!(stack.alpha(), alpha);
        assert_eq!(stack.beta(), beta);
    }

    #[test]
    fn test_stack_bucket_pop_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(move || Uuid::nil().into());

        let mut stack = Stack::new(Data::default(), Box::new(ops)).unwrap();

        assert_none!(stack.pop());
    }

    #[test]
    fn test_stack_bucket_pop() {
        let doc = Document {
            id: DocumentId::from_u128(u128::MIN),
            stack_id: Id(Uuid::nil()),
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        let data = Data::new(0.01, 0.99, vec![doc.clone(), doc]).unwrap();
        let mut ops = MockOps::new();
        ops.expect_id().returning(move || Uuid::nil().into());
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();

        assert_some!(stack.pop());
        assert_some!(stack.pop());
        assert_none!(stack.pop());
    }

    #[test]
    fn test_stack_bucket_is_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(move || Uuid::nil().into());
        let data = Data::default();
        let stack = Stack::new(data, Box::new(ops)).unwrap();

        assert!(stack.is_empty());

        let doc = Document {
            id: DocumentId::from_u128(u128::MIN),
            stack_id: Id(Uuid::nil()),
            rank: usize::default(),
            title: String::default(),
            snippet: String::default(),
            url: String::default(),
            domain: String::default(),
            smbert_embedding: Embedding(arr1(&[])),
        };

        let mut ops = MockOps::new();
        ops.expect_id().returning(move || Uuid::nil().into());
        let data = Data::new(1., 1., vec![doc]).unwrap();
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();

        assert!(!stack.is_empty());
        stack.pop();
        assert!(stack.is_empty());
    }
}
