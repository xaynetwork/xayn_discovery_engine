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
use xayn_discovery_engine_ai::{CoiPoint, GenericError, KeyPhrase, UserInterests};
use xayn_discovery_engine_providers::{GenericArticle, Market};

use crate::{
    config::StackConfig as Config,
    document::{Document, HistoricDocument, Id as DocumentId, UserReaction},
    mab::Bucket,
    stack::filters::filter_too_similar,
};

mod data;
pub(crate) mod exploration;
pub(crate) mod filters;
pub(crate) mod ops;

pub(crate) use self::{data::Data, ops::NewItemsError};
pub use self::{
    exploration::Stack as Exploration,
    ops::{
        breaking::BreakingNews,
        personalized::PersonalizedNews,
        trusted::TrustedNews,
        BoxedOps,
        Ops,
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

    /// Failed to select new items: {0}.
    Selection(#[from] exploration::Error),
}

/// Id of a stack.
///
/// `Id` is used to connect [`Ops`](ops::Ops) with the corresponding data of a stack.
#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, From, Display,
)]
#[repr(transparent)]
#[cfg_attr(
    feature = "storage",
    derive(sqlx::Type, sqlx::FromRow),
    sqlx(transparent)
)]
pub struct Id(Uuid);

impl Id {
    #[cfg(test)]
    pub fn new_random() -> Self {
        Id(Uuid::new_v4())
    }

    pub const fn nil() -> Self {
        Id(Uuid::nil())
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_nil()
    }

    pub fn name(&self) -> Option<&'static str> {
        match self {
            id if id == &exploration::Stack::id() => Some(exploration::Stack::name()),
            id if id == &ops::breaking::BreakingNews::id() => {
                Some(ops::breaking::BreakingNews::name())
            }
            id if id == &ops::personalized::PersonalizedNews::id() => {
                Some(ops::personalized::PersonalizedNews::name())
            }
            id if id == &ops::trusted::TrustedNews::id() => Some(ops::trusted::TrustedNews::name()),
            _ => None,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct Stack {
    pub(crate) data: Data,
    #[derivative(Debug = "ignore")]
    pub(crate) ops: BoxedOps,
    config: Config,
}

impl Stack {
    /// Create a new `Stack` with the given [`Data`] and customized [`Ops`].
    pub(crate) fn new(data: Data, ops: BoxedOps, config: Config) -> Result<Self, Error> {
        Self::validate_documents_stack_id(&data.documents, ops.id())?;

        Ok(Self { data, ops, config })
    }

    /// [`Id`] of this `Stack`.
    pub(crate) fn id(&self) -> Id {
        self.ops.id()
    }

    /// Updates the internal documents with the new one and returns an updated [`Stack`].
    pub(crate) fn update(
        &mut self,
        user_interests: &UserInterests,
        ranker: impl FnOnce(&mut [Document]),
        new_documents: &[Document],
    ) -> Result<(), Error> {
        Self::validate_documents_stack_id(new_documents, self.ops.id())?;

        let documents = self
            .ops
            .merge(&self.data.documents, new_documents)
            .map_err(Error::Merge)?;
        let mut documents = filter_too_similar(
            documents,
            user_interests.negative.iter().map(|coi| coi.point().view()),
            self.config.max_negative_similarity,
        );
        ranker(&mut documents);
        self.data.documents = documents;
        self.data.documents.reverse();

        Ok(())
    }

    /// Rank the internal documents.
    ///
    /// This is useful when the [`Engine`] has been updated.
    ///
    /// [`Engine`]: crate::engine::Engine
    pub(crate) fn rank(&mut self, ranker: impl FnOnce(&mut [Document])) {
        ranker(&mut self.data.documents);
        self.data.documents.reverse();
    }

    /// Updates the relevance of the Stack based on the user feedback.
    pub(crate) fn update_relevance(
        &mut self,
        reaction: UserReaction,
        max_reactions: usize,
        incr_reactions: f32,
    ) {
        update_relevance(&mut self.data, reaction, max_reactions, incr_reactions);
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

    /// Removes documents whose source is an excluded source.
    pub(crate) fn prune_by_excluded_sources(&mut self, excluded_sources: &HashSet<String>) {
        self.data
            .documents
            .retain(|doc| !excluded_sources.contains(&doc.resource.source_domain));
    }

    pub(crate) fn drain_documents(&mut self) -> std::vec::Drain<'_, Document> {
        self.data.documents.drain(..)
    }
}

pub(crate) fn update_relevance(
    data: &mut Data,
    reaction: UserReaction,
    max_reactions: usize,
    incr_reactions: f32,
) {
    match reaction {
        UserReaction::Positive => data.likes += incr_reactions,
        UserReaction::Negative => data.dislikes += incr_reactions,
        UserReaction::Neutral => {}
    }
    let num_reactions = data.likes + data.dislikes;
    #[allow(clippy::cast_precision_loss)] // value should be small enough
    let max_reactions = max_reactions as f32;
    if num_reactions <= max_reactions {
        data.alpha = data.likes;
        data.beta = data.dislikes;
    } else {
        data.alpha = data.likes * max_reactions / num_reactions;
        data.beta = data.dislikes * max_reactions / num_reactions;

        if data.alpha < 1. {
            data.alpha = 1.;
            data.beta = max_reactions - 1.;
        }

        if data.beta < 1. {
            data.alpha = max_reactions - 1.;
            data.beta = 1.;
        }
    }
    data.alpha = (10. * data.alpha).round() / 10.;
    data.beta = (10. * data.beta).round() / 10.;
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

        assert!(f(&[]).is_ok());
        assert!(f(&[doc_1.clone()]).is_ok());
        assert!(f(&[doc_1, doc_2]).is_ok());
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
            assert!(matches!(
                f(docs),
                Err(Error::InvalidDocument { document_id, document_stack_id, stack_id})
                    if document_id == doc_ko.id && document_stack_id == doc_ko.stack_id && stack_id == stack_id_ok,
            ));
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
        assert!(Stack::new(Data::default(), Box::new(ops), Config::default()).is_ok());
    }

    #[test]
    fn test_stack_new_valid_documents() {
        let stack_id = Id::default();

        assert_valid_document(
            |docs| {
                let mut ops = MockOps::new();
                ops.expect_id().returning(move || stack_id);

                let data = Data::new(0.01, 0.99, docs.to_vec()).unwrap();
                Stack::new(data, Box::new(ops), Config::default())
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
                Stack::new(data, Box::new(ops), Config::default())
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
        let stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();

        assert_eq!(stack.alpha(), alpha);
        assert_eq!(stack.beta(), beta);
    }

    #[test]
    fn test_stack_bucket_pop_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);

        let mut stack = Stack::new(Data::default(), Box::new(ops), Config::default()).unwrap();

        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_stack_bucket_pop() {
        let doc = Document::default();

        let data = Data::new(0.01, 0.99, vec![doc.clone(), doc]).unwrap();
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let mut stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();

        assert!(stack.pop().is_some());
        assert!(stack.pop().is_some());
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_stack_bucket_is_empty() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();

        assert!(stack.is_empty());

        let doc = Document::default();

        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::new(1., 1., vec![doc]).unwrap();
        let mut stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();

        assert!(!stack.is_empty());
        stack.pop();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_feedback_reaction_positive() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Positive, 10, 1.);

        assert_approx_eq!(f32, alpha + 1., stack.alpha());
        assert_approx_eq!(f32, beta, stack.beta());
    }

    #[test]
    fn test_stack_feedback_reaction_negative() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Negative, 10, 1.);

        assert_approx_eq!(f32, beta + 1., stack.beta());
        assert_approx_eq!(f32, alpha, stack.alpha());
    }

    #[test]
    fn test_stack_feedback_reaction_neutral() {
        let mut ops = MockOps::new();
        ops.expect_id().returning(Id::default);
        let data = Data::default();
        let mut stack = Stack::new(data, Box::new(ops), Config::default()).unwrap();
        let alpha = stack.alpha();
        let beta = stack.beta();

        stack.update_relevance(UserReaction::Neutral, 10, 1.);

        assert_approx_eq!(f32, beta, stack.beta());
        assert_approx_eq!(f32, alpha, stack.alpha());
    }
}
