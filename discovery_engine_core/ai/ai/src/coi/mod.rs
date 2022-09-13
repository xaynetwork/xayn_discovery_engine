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

pub(crate) mod config;
pub(crate) mod context;
pub(crate) mod point;
pub(crate) mod stats;
pub(crate) mod system;

use derive_more::From;
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::embedding::Embedding;

/// A unique identifier of a `CoI`.
#[repr(transparent)] // needed for FFI
#[derive(
    Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Serialize, Deserialize, From,
)]
pub struct CoiId(Uuid);

#[derive(Debug, Display, Error)]
pub(crate) enum CoiError {
    /// A key phrase is empty
    EmptyKeyPhrase,
    /// A key phrase has non-finite embedding values: {0:#?}
    NonFiniteKeyPhrase(Embedding),
}

#[cfg(test)]
mod tests {
    use xayn_discovery_engine_test_utils::uuid::mock_uuid;

    use super::*;

    impl CoiId {
        /// Creates a mocked `CoI` id from a mocked UUID, cf. [`mock_uuid()`].
        pub(crate) const fn mocked(sub_id: usize) -> Self {
            Self(mock_uuid(sub_id))
        }
    }
}
