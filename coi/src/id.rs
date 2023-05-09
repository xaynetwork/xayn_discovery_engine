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

use derive_more::{AsRef, From};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A unique identifier of a `CoI`.
#[derive(
    Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord, Serialize, Deserialize, From, AsRef,
)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct CoiId(Uuid);

impl CoiId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::uuid::mock_uuid;

    use super::*;

    impl CoiId {
        /// Creates a mocked `CoI` id from a mocked UUID, cf. [`mock_uuid()`].
        pub(crate) const fn mocked(sub_id: usize) -> Self {
            Self(mock_uuid(sub_id))
        }
    }
}
