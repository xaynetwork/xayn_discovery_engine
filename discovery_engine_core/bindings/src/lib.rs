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

//! FFI and logic bindings to `discovery_engine_core`.

#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused_qualifications,
    unsafe_op_in_unsafe_fn
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

pub mod async_bindings;
pub mod types;

#[async_bindgen::api]
impl AsyncCore {
    /// Initializes the engine.
    pub async fn initialize(
        config: Box<core::InitConfig>,
        state: &Option<Vec<u8>>,
    ) -> Box<Result<core::LockedEngine, String>> {
        Box::new(
            core::Engine::from_config(*config, state.as_deref())
                .await
                .map(|engine| core::LockedEngine(tokio::sync::Mutex::new(engine)))
                .map_err(|error| error.to_string()),
        )
    }

    /// Serializes the engine.
    pub async fn serialize(engine: &core::LockedEngine) -> Box<Result<Vec<u8>, String>> {
        Box::new(
            engine
                .0
                .lock()
                .await
                .serialize()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the markets.
    #[allow(clippy::box_vec)]
    pub async fn set_markets(
        engine: &core::LockedEngine,
        markets: Box<Vec<core::Market>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .0
                .lock()
                .await
                .set_markets(*markets)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets feed documents.
    pub async fn get_feed_documents(
        engine: &core::LockedEngine,
        max_documents: u32,
    ) -> Box<Result<Vec<core::document::Document>, String>> {
        Box::new(
            engine
                .0
                .lock()
                .await
                .get_feed_documents(max_documents as usize)
                .await
                .map_err(|error| error.to_string()),
        )
    }
}
