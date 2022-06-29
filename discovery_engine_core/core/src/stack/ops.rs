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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // check that Ops is object safe
    #[test]
    fn check_ops_obj_safe() {
        let _: Option<&dyn Ops> = None;
    }
}
