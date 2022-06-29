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

use async_trait::async_trait;
use displaydoc::Display;
#[cfg(test)]
use mockall::automock;
use thiserror::Error;

use xayn_discovery_engine_ai::{GenericError, KeyPhrase};
use xayn_discovery_engine_providers::{GenericArticle, Market};

use crate::{
    document::{Document, HistoricDocument},
    stack::Id,
};

pub(crate) mod breaking;
mod common;
pub(crate) mod personalized;
pub(crate) mod trusted;

/// When asking for new articles the stack could be in a not ready state
/// to ask for them.
#[derive(Error, Debug, Display)]
pub enum NewItemsError {
    /// The stack is not ready to retrieve new items.
    NotReady,
    /// Retrieving new items error: {0}
    Error(#[from] GenericError),
}

/// Operations to customize the behaviour of a stack.
///
/// Each stack can get and select new items using different sources
/// or different strategies.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Ops {
    /// Get the id for this set of operations.
    ///
    /// Only one stack with a given id can be added to [`Engine`](crate::engine::Engine).
    /// This method must always return the same value for a given implementation.
    fn id(&self) -> Id;

    /// Returns new items that could be added to the stack.
    ///
    /// Personalized key phrases can be optionally used to return items
    /// tailored to the user's interests.
    async fn new_items(
        &self,
        key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
        market: &Market,
    ) -> Result<Vec<GenericArticle>, NewItemsError>;

    /// Returns if `[new_items]` needs the key phrases to work.
    fn needs_key_phrases(&self) -> bool;

    /// Merge stacked and new items.
    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError>;
}

/// Convenience type that boxes an [`Ops`] and adds [`Send`] and [`Sync`].
pub type BoxedOps = Box<dyn Ops + Send + Sync>;

#[async_trait]
impl Ops for BoxedOps {
    fn id(&self) -> Id {
        self.as_ref().id()
    }

    async fn new_items(
        &self,
        key_phrases: &[KeyPhrase],
        history: &[HistoricDocument],
        stack: &[Document],
        market: &Market,
    ) -> Result<Vec<GenericArticle>, NewItemsError> {
        self.as_ref()
            .new_items(key_phrases, history, stack, market)
            .await
    }

    fn needs_key_phrases(&self) -> bool {
        self.as_ref().needs_key_phrases()
    }

    fn merge(&self, stack: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        self.as_ref().merge(stack, new)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn test_ops_trait_is_object_safe() {
        let _: Option<&dyn Ops> = None;
        #[allow(clippy::let_underscore_drop)]
        let _: Option<BoxedOps> = None;
    }
}
