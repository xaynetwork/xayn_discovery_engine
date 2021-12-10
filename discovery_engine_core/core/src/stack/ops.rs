#[cfg(test)]
use mockall::automock;

use crate::{document::Document, engine::GenericError, stack::Id};

/// Operations to customize the behaviour of a `[Stack]`.
///
/// Each stack can get and select new items using different sources
/// or different strategies.
#[cfg_attr(test, automock)]
pub trait Ops {
    /// Get the id for this set of operations.
    ///
    /// Only one stack with a given id can be added to [`Engine`].
    /// This method must always return the same value for a given implementation.
    fn id(&self) -> Id;

    /// Returns new items that could be added to the `[Stack]`.
    ///
    /// Personalized key phrases can be optionally used to return items
    /// tailored to the user's interests.
    fn new_items(&self, keyphrases: &[String]) -> Result<Vec<Document>, GenericError>;

    /// Merge current and new items.
    fn merge(&self, current: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    // check that Ops is object safe
    #[test]
    fn check_ops_obj_safe() {
        let _: Option<&dyn Ops> = None;
    }
}
