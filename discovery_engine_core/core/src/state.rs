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

use std::collections::HashMap;

use serde::Deserialize;
use xayn_discovery_engine_ai::{GenericError, KeyPhrases, UserInterests};

use crate::{
    engine::{Engine, Error},
    stack::{exploration::Stack as Exploration, Data, Id},
};

const STATE_VERSION: u8 = 2;

impl Engine {
    /// Serializes the state of the engine.
    // TODO: remove the ffi for this method and reduce its visibility after DB migration
    pub async fn serialize(&self) -> Result<Vec<u8>, Error> {
        let stacks = self.stacks.read().await;
        let mut stacks_data = stacks
            .iter()
            .map(|(id, stack)| (id, &stack.data))
            .collect::<HashMap<_, _>>();
        let exploration_stack_id = Exploration::id();
        stacks_data.insert(&exploration_stack_id, &self.exploration_stack.data);
        let state = &(stacks_data, &self.user_interests, &self.key_phrases);

        // version is encoded in the first byte
        let size = 1 + bincode::serialized_size(state).map_err(Error::Serialization)?;
        #[allow(clippy::cast_possible_truncation)] // bounded by architecture
        let mut bytes = Vec::with_capacity(size as usize);
        bytes.push(STATE_VERSION);
        bincode::serialize_into(&mut bytes, state).map_err(Error::Serialization)?;

        #[cfg(feature = "storage")]
        {
            self.storage.state().store(bytes).await?;
            Ok(Vec::new())
        }

        #[cfg(not(feature = "storage"))]
        Ok(bytes)
    }

    pub(crate) fn deserialize(
        bytes: &[u8],
    ) -> Result<(HashMap<Id, Data>, UserInterests, KeyPhrases), Error> {
        match bytes.get(0) {
            Some(version) if *version < STATE_VERSION => Ok(Default::default()),
            Some(&STATE_VERSION) => {
                bincode::deserialize(&bytes[1..]).map_err(Error::Deserialization)
            }
            Some(version) => Err(GenericError::from(format!(
                "Unsupported serialized data. Found version {} expected {}",
                *version, STATE_VERSION,
            ))
            .into()),
            None => Err(GenericError::from("Empty serialized data").into()),
        }
        // Serialized data could be the partially unversioned data we had before
        .or_else(|error| {
            #[derive(Deserialize)]
            struct SerializedStackState(Vec<u8>);
            #[derive(Deserialize)]
            struct SerializedCoiSystemState(Vec<u8>);
            #[derive(Deserialize)]
            struct SerializedState {
                stacks: SerializedStackState,
                coi: SerializedCoiSystemState,
            }

            bincode::deserialize::<SerializedState>(bytes)
                .and_then(|state| {
                    bincode::deserialize::<HashMap<Id, Data>>(&state.stacks.0)
                        .map(|stacks| (stacks, state))
                })
                .and_then(|(stacks, state)| {
                    match state.coi.0.get(0) {
                        Some(&0) => Ok((stacks, UserInterests::default(), KeyPhrases::default())),
                        Some(&1) => {
                            #[derive(Deserialize)]
                            struct CoiSystemState {
                                user_interests: UserInterests,
                                key_phrases: KeyPhrases,
                            }

                            bincode::deserialize::<CoiSystemState>(&state.coi.0[1..])
                                .map(|coi| (stacks, coi.user_interests, coi.key_phrases))
                        }
                        // Serialized data could be the unversioned data we had before
                        _ => bincode::deserialize::<UserInterests>(bytes)
                            .map(|user_interests| (stacks, user_interests, KeyPhrases::default())),
                    }
                })
                .map_err(|_| error)
        })
    }
}
