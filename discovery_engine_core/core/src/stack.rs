// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Export types to customize the behaviour of a stack.

use derivative::Derivative;
use derive_more::{Display, From};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    document::{Document, Id as DocumentId, UserReaction},
    engine::{GenericError, Ranker},
    mab::Bucket,
};

mod data;
mod ops;

pub(crate) use data::Data;
pub use ops::Ops;

/// Errors that could occur while manipulating a stack.
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
#[cfg_attr(test, derive(Default))]
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
    pub(crate) fn new(data: Data, ops: BoxedOps) -> Result<Self, Error> {
        Self::validate_documents_stack_id(&data.documents, ops.id())?;

        Ok(Self { data, ops })
    }

    /// [`Id`] of this `Stack`.
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> Id {
        self.ops.id()
    }

    /// Updates the internal documents with the new one and returns an updated [`Stack`].
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

    /// Rank the internal documents.
    ///
    /// This is useful when the [`Ranker`] has been updated.
    pub(crate) fn rank<R: Ranker>(&mut self, ranker: &R) -> Result<(), Error> {
        ranker
            .rank(&mut self.data.documents)
            .map_err(Error::Ranking)
    }

    /// Updates the relevance of the Stack based on the user feedback.
    #[allow(dead_code)]
    pub(crate) fn update_relevance(&mut self, reaction: UserReaction) {
        // to avoid making the distribution too skewed
        const MAX_BETA_PARAMS: f32 = 1000.;

        fn incr(value: &mut f32) {
            if *value < MAX_BETA_PARAMS {
                (*value) += 1.;
            }
        }

        match reaction {
            UserReaction::Positive => incr(&mut self.data.alpha),
            UserReaction::Negative => incr(&mut self.data.beta),
            UserReaction::Neutral => (),
        }
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
    use std::fmt::Debug;

    use claim::{assert_matches, assert_none, assert_ok, assert_some};
    use uuid::Uuid;
    // TODO use our own when exposed as a crate
    use float_cmp::approx_eq;

    use crate::{document::Id as DocumentId, stack::ops::MockOps};

    use super::*;

    // assert that `f` returns ok if the argument contains only documents valid `stack_id`
    fn assert_valid_document<T, F>(f: F, stack_id: Id)
    where
        T: Debug,
        F: Fn(&[Document]) -> Result<T, Error>,
    {
        let doc_1 = Document {
            stack_id,
            ..Document::default()
        };

        let doc_2 = Document {
            id: DocumentId::from_u128(1),
            stack_id,
            ..Document::default()
        };

        assert_ok!(f(&[]));
        assert_ok!(f(&[doc_1.clone()]));
        assert_ok!(f(&[doc_1, doc_2]));
    }

    // assert that `f` returns an error if the argument contains a Document with an invalid `stack_id`
    fn assert_invalid_document<T, F>(f: F, stack_id_ok: Id)
    where
        T: Debug,
        F: Fn(&[Document]) -> Result<T, Error>,
    {
        let stack_id_ko = Uuid::from_u128(stack_id_ok.0.as_u128() + 1).into();
        let doc_id_ko = DocumentId::from_u128(1);

        let doc_ok = Document {
            stack_id: stack_id_ok,
            ..Document::default()
        };

        let doc_ko = Document {
            id: doc_id_ko,
            stack_id: stack_id_ko,
            ..Document::default()
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
        let stack_id = Id::default();

        assert_valid_document(
            |docs| Stack::validate_documents_stack_id(docs, stack_id),
            stack_id,
        );
    }

    #[test]
    fn test_stack_validate_documents_stack_id_ko() {
        let stack_id = Id::default();

        assert_invalid_document(
            |docs| Stack::validate_documents_stack_id(docs, stack_id),
            stack_id,
        );
    }

    #[test]
    fn test_stack_new_from_default() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        assert_ok!(Stack::new(Data::default(), Box::new(ops)));
    }

    #[test]
    fn test_stack_new_valid_documents() {
        let stack_id = Id::default();

        assert_valid_document(
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
        let stack_id = Id::default();

        assert_invalid_document(
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
        ops.expect_id().returning(Id::default);

        let data = Data::new(alpha, beta, vec![]).unwrap();
        let stack = Stack::new(data, Box::new(ops)).unwrap();

        assert_eq!(stack.alpha(), alpha);
        assert_eq!(stack.beta(), beta);
    }

    #[test]
    fn test_stack_bucket_pop_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);

        let mut stack = Stack::new(Data::default(), Box::new(ops)).unwrap();

        assert_none!(stack.pop());
    }

    #[test]
    fn test_stack_bucket_pop() {
        let doc = Document::default();

        let data = Data::new(0.01, 0.99, vec![doc.clone(), doc]).unwrap();
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();

        assert_some!(stack.pop());
        assert_some!(stack.pop());
        assert_none!(stack.pop());
    }

    #[test]
    fn test_stack_bucket_is_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let stack = Stack::new(data, Box::new(ops)).unwrap();

        assert!(stack.is_empty());

        let doc = Document::default();

        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::new(1., 1., vec![doc]).unwrap();
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();

        assert!(!stack.is_empty());
        stack.pop();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_feedback_reaction_positive() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Positive);

        approx_eq!(f32, alpha + 1., stack.alpha());
        approx_eq!(f32, beta, stack.beta());
    }

    #[test]
    fn test_stack_feedback_reaction_negative() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Negative);

        approx_eq!(f32, beta + 1., stack.beta());
        approx_eq!(f32, alpha, stack.alpha());
    }

    #[test]
    fn test_stack_feedback_reaction_neutral() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops)).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Neutral);

        approx_eq!(f32, beta, stack.beta());
        approx_eq!(f32, alpha, stack.alpha());
    }
}
