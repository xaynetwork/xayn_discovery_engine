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

//! Storage specific interfaces which we always need as they appear in the public api.
//FIXME merge with `crate::storage` once the feature flag is gone.

#[cfg_attr(test, derive(Clone))]
pub struct DartMigrationData {
    pub engine_state: Option<Vec<u8>>,
    pub trusted_sources: Vec<String>,
    pub excluded_sources: Vec<String>,
}

/// Hint about what was done during db init.
pub enum InitDbHint {
    /// Hint to use if nothing special happened during init.
    NormalInit,
    /// A new db was created, there was no db beforehand.
    #[cfg(feature = "storage")]
    NewDbCreated,
    /// There was a db but we could not open it so we deleted it and created a new one.
    #[cfg(feature = "storage")]
    DbOverwrittenDueToErrors(crate::storage::Error),
}
