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

use std::collections::HashSet;

use derivative::Derivative;
use derive_more::{Display, From};
use displaydoc::Display as DisplayDoc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;
use xayn_discovery_engine_ai::{CoiSystem, GenericError, KeyPhrase, UserInterests};
use xayn_discovery_engine_providers::{GenericArticle, Market};

use crate::{
    document::{Document, HistoricDocument, Id as DocumentId, UserReaction},
    mab::Bucket,
};

mod data;
pub(crate) mod exploration;
pub(crate) mod filters;
pub(crate) mod ops;

pub use self::ops::{BoxedOps, Ops};
pub(crate) use self::{
    data::Data,
    ops::{
        breaking::BreakingNews,
        personalized::PersonalizedNews,
        trusted::TrustedNews,
        NewItemsError,
    },
};

/// Errors that could occur while manipulating a stack.
#[derive(Error, Debug, DisplayDoc)]
pub enum Error {
    /// Failed to merge current documents with new ones.
    Merge(#[source] GenericError),

    /// [`Document`] {document_id} has stack id {document_stack_id} instead of {stack_id}.
    InvalidDocument {
        /// [`DocumentId`] of the offending document.
        document_id: DocumentId,
        /// [`StackId`](Id) of the document.
        document_stack_id: Id,
        /// [`StackId`](Id) of the the current stack.
        stack_id: Id,
    },

    /// Failed to get new items: {0}.
    New(#[source] NewItemsError),

    /// Failed to filter: {0}.
    Filter(#[source] GenericError),

    /// Missing the document history to update a stack.
    NoHistory,

    /// Failed to select new items: {0}.
    Selection(#[from] exploration::Error),
}

/// Id of a stack.
///
/// `Id` is used to connect [`Ops`](ops::Ops) with the corresponding data of a stack.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, From, Display)]
#[repr(transparent)]
#[cfg_attr(test, derive(Default))]
pub struct Id(Uuid);

impl Id {
    #[cfg(test)]
    pub fn new_random() -> Self {
        Id(Uuid::new_v4())
    }

    pub(crate) const fn nil() -> Self {
        Id(Uuid::nil())
    }

    pub(crate) fn is_nil(&self) -> bool {
        self.0.is_nil()
    }
}

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
    pub(crate) fn id(&self) -> Id {
        self.ops.id()
    }

    /// Updates the internal documents with the new one and returns an updated [`Stack`].
    pub(crate) fn update(
        &mut self,
        new_documents: &[Document],
        coi: &CoiSystem,
        user_interests: &UserInterests,
    ) -> Result<(), Error> {
        Self::validate_documents_stack_id(new_documents, self.ops.id())?;

        let mut items = self
            .ops
            .merge(&self.data.documents, new_documents)
            .map_err(Error::Merge)?;
        coi.rank(&mut items, user_interests);
        self.data.documents = items;
        self.data.documents.reverse();
        Ok(())
    }

    /// Rank the internal documents.
    ///
    /// This is useful when the [`Engine`] has been updated.
    ///
    /// [`Engine`]: crate::engine::Engine
    pub(crate) fn rank(&mut self, coi: &CoiSystem, user_interests: &UserInterests) {
        coi.rank(&mut self.data.documents, user_interests);
        self.data.documents.reverse();
    }

    /// Updates the relevance of the Stack based on the user feedback.
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

    pub(crate) fn len(&self) -> usize {
        self.data.documents.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.documents.is_empty()
    }

    /// Returns a list of new articles.
    pub(crate) async fn new_items(
        &self,
        key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        market: &Market,
    ) -> Result<Vec<GenericArticle>, Error> {
        self.ops
            .new_items(key_phrases, history, &self.data.documents, market)
            .await
            .map_err(Error::New)
    }

    /// Filter documents according to whether their source matches one in `sources`.
    /// The flag `exclude` indicates whether to ex/include such documents.
    pub(crate) fn prune_by_sources(&mut self, sources: &HashSet<String>, exclude: bool) {
        self.data
            .documents
            .retain(|doc| sources.contains(&doc.resource.source_domain) ^ exclude);
    }

    pub(crate) fn drain_documents(&mut self) -> std::vec::Drain<'_, Document> {
        self.data.documents.drain(..)
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
        self.is_empty()
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
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    use crate::stack::ops::MockOps;

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
            id: Uuid::from_u128(1).into(),
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
        let doc_id_ko = Uuid::from_u128(1).into();

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

        assert_approx_eq!(f32, alpha + 1., stack.alpha());
        assert_approx_eq!(f32, beta, stack.beta());
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

        assert_approx_eq!(f32, beta + 1., stack.beta());
        assert_approx_eq!(f32, alpha, stack.alpha());
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

        assert_approx_eq!(f32, beta, stack.beta());
        assert_approx_eq!(f32, alpha, stack.alpha());
    }
}
