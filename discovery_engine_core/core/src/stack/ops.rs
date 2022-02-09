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

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError},
    ranker::Ranker,
    stack::Id,
};
use xayn_ai::ranker::KeyPhrase;

/// Operations to customize the behaviour of a stack.
///
/// Each stack can get and select new items using different sources
/// or different strategies.
#[async_trait]
pub trait Ops {
    /// Get the id for this set of operations.
    ///
    /// Only one stack with a given id can be added to [`Engine`](crate::engine::Engine).
    /// This method must always return the same value for a given implementation.
    fn id(&self) -> Id;

    /// Configure the operations from endpoint settings.
    fn configure(&self, config: EndpointConfig);

    /// Returns new items that could be added to the stack.
    ///
    /// Personalized key phrases can be optionally used to return items
    /// tailored to the user's interests.
    async fn new_items<'a>(
        &self,
        key_phrases: &[KeyPhrase],
        ranker: &'a (dyn Ranker + Sync),
    ) -> Result<Vec<Document>, GenericError>;

    /// Merge current and new items.
    fn merge(&self, current: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError>;
}

#[cfg(test)]
pub(crate) mod tests {
    use mockall::mock;

    use super::*;

    // mocking introduces an additional distinct lifetime without the alias
    pub(crate) type R<'a> = &'a (dyn Ranker + Sync);

    mock! {
        pub(crate) Ops {}

        #[async_trait]
        impl Ops for Ops {
            fn id(&self) -> Id;

            fn configure(&self, config: EndpointConfig);

            async fn new_items<'a>(
                &self,
                key_phrases: &[KeyPhrase],
                ranker: R<'a>,
            ) -> Result<Vec<Document>, GenericError>;

            fn merge(
                &self,
                current: &[Document],
                new: &[Document],
            ) -> Result<Vec<Document>, GenericError>;
        }
    }

    // check that Ops is object safe
    #[test]
    fn check_ops_obj_safe() {
        let _: Option<&dyn Ops> = None;
    }
}
