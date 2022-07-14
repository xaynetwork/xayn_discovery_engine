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

use serde::{Deserialize, Serialize};

use crate::{
    coi::{key_phrase::KeyPhrases, point::UserInterests},
    error::GenericError,
};

const STATE_VERSION: u8 = 1;

/// The state of the coi system.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// The learned user interests.
    pub user_interests: UserInterests,

    /// Key phrases.
    pub key_phrases: KeyPhrases,
}

impl State {
    /// Creates a byte representation of the internal state of the ranker.
    pub fn serialize(&self) -> Result<Vec<u8>, GenericError> {
        let size = bincode::serialized_size(self)? + 1;
        #[allow(clippy::cast_possible_truncation)] // bounded by architecture
        let mut serialized = Vec::with_capacity(size as usize);
        // version is encoded in the first byte
        serialized.push(STATE_VERSION);
        bincode::serialize_into(&mut serialized, self)?;

        Ok(serialized)
    }

    /// Sets the serialized state to use.
    ///
    /// # Errors
    ///
    /// Fails if the state cannot be deserialized.
    pub fn deserialize(bytes: impl AsRef<[u8]>) -> Result<Self, GenericError> {
        let bytes = bytes.as_ref();

        let state = match bytes[0] {
            version if version < STATE_VERSION => Ok(State::default()),
            STATE_VERSION => bincode::deserialize(&bytes[1..]).map_err(Into::into),
            version => Err(format!(
                "Unsupported serialized data. Found version {} expected {}",
                version, STATE_VERSION,
            )
            .into()),
        }
        .or_else(|e: GenericError|
                  // Serialized data could be the unversioned data we had before
                  bincode::deserialize(bytes).map(|user_interests|
                                                  State {
                                                      user_interests,
                                                      ..State::default()
                                                  }
                  ).map_err(|_| e))?;

        Ok(state)
    }

    /// Resets the AI state but not configurations.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
