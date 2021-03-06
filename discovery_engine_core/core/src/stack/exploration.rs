// Copyright 2022 Xayn AG
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

//! Exploration stack.

use std::collections::HashSet;
use uuid::uuid;
use xayn_discovery_engine_ai::{CoiSystem, UserInterests};

use crate::{
    config::ExplorationConfig as Config,
    document::{Document, UserReaction},
    mab::Bucket,
    stack::{self, exploration::selection::document_selection, Data, Id},
};

mod selection;

pub(crate) use self::selection::Error;

#[derive(Debug)]
pub(crate) struct Stack {
    pub(crate) data: Data,
    config: Config,
}

impl Stack {
    /// Create a new `Stack` with the given [`Data`].
    pub(crate) fn new(data: Data, config: Config) -> Result<Self, stack::Error> {
        Self::validate_documents_stack_id(&data.documents, Stack::id())?;
        Ok(Self { data, config })
    }

    /// [`Id`] of this `Stack`.
    pub(crate) const fn id() -> Id {
        Id(uuid!("77cf9280-bb93-4158-b660-8732927e0dcc"))
    }

    /// Updates the internal documents with the new one and returns an updated [`Stack`].
    pub(crate) fn update(
        &mut self,
        new_documents: &[Document],
        coi: &CoiSystem,
        user_interests: &UserInterests,
    ) -> Result<(), stack::Error> {
        if user_interests.positive.is_empty() && user_interests.negative.is_empty() {
            // we are not ready to run the exploration stack
            return Ok(());
        }

        let new_documents = new_documents.iter().cloned().map(|mut doc| {
            doc.stack_id = Self::id();
            doc
        });

        let documents = self
            .data
            .documents
            .iter()
            .cloned()
            .chain(new_documents)
            .collect();

        let mut items = document_selection(
            &user_interests.positive,
            &user_interests.negative,
            documents,
            &self.config,
        )?;
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
    fn validate_documents_stack_id(
        documents: &[Document],
        stack_id: Id,
    ) -> Result<(), stack::Error> {
        if let Some(doc) = documents.iter().find(|doc| doc.stack_id != stack_id) {
            return Err(stack::Error::InvalidDocument {
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
