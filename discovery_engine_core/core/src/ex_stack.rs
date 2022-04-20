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
use std::collections::HashSet;
use uuid::Uuid;

use crate::{
    document::{Document, UserReaction},
    mab::Bucket,
    ranker::Ranker,
    stack::{Data, Error, Id},
};

#[derive(Derivative)]
#[derivative(Debug)]
pub(crate) struct ExplorationStack {
    pub(crate) data: Data,
}

impl ExplorationStack {
    /// Create a new `Stack` with the given [`Data`] and customized [`Ops`].
    pub(crate) fn new(data: Data) -> Result<Self, Error> {
        Self::validate_documents_stack_id(&data.documents, ExplorationStack::id())?;

        Ok(Self { data })
    }

    /// [`Id`] of this `Stack`.
    pub(crate) fn id() -> Id {
        Uuid::parse_str("77cf9280-bb93-4158-b660-8732927e0dcc").unwrap(/* valid uuid */).into()
    }

    /// Updates the internal documents with the new one and returns an updated [`Stack`].
    pub(crate) fn update(
        &mut self,
        new_documents: &[Document],
        ranker: &mut impl Ranker,
    ) -> Result<(), Error> {
        Self::validate_documents_stack_id(new_documents, ExplorationStack::id())?;

        self.data.documents.extend(new_documents.to_owned());
        self.rank(ranker)
    }

    /// Rank the internal documents.
    ///
    /// This is useful when the [`Ranker`] has been updated.
    pub(crate) fn rank(&mut self, ranker: &mut impl Ranker) -> Result<(), Error> {
        ranker
            .rank(&mut self.data.documents)
            .map_err(Error::Ranking)
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

    pub(crate) fn is_empty(&self) -> bool {
        self.data.documents.is_empty()
    }

    /// Filter documents according to whether their source matches one in `sources`.
    /// The flag `exclude` indicates whether to ex/include such documents.
    pub(crate) fn prune_by_sources(&mut self, sources: &HashSet<String>, exclude: bool) {
        self.data
            .documents
            .retain(|doc| sources.contains(&doc.resource.source_domain) ^ exclude);
    }
}

impl Bucket<Document> for ExplorationStack {
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
