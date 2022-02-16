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
impl XaynDiscoveryEngineAsyncFfi {
    /// Initializes the engine.
    #[allow(clippy::box_vec)]
    pub async fn initialize(
        config: Box<xayn_discovery_engine_core::InitConfig>,
        state: Option<Box<Vec<u8>>>,
    ) -> Box<Result<crate::types::engine::SharedEngine, String>> {
        Box::new(
            xayn_discovery_engine_core::Engine::from_config(
                *config,
                state.as_deref().map(Vec::as_slice),
            )
            .await
            .map(|engine| tokio::sync::Mutex::new(engine).into())
            .map_err(|error| error.to_string()),
        )
    }

    /// Serializes the engine.
    pub async fn serialize(
        engine: &crate::types::engine::SharedEngine,
    ) -> Box<Result<Vec<u8>, String>> {
        Box::new(
            engine
                .as_ref()
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
        engine: &crate::types::engine::SharedEngine,
        markets: Box<Vec<xayn_discovery_engine_core::Market>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_markets(*markets)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets feed documents.
    pub async fn get_feed_documents(
        engine: &crate::types::engine::SharedEngine,
        max_documents: usize,
    ) -> Box<Result<Vec<xayn_discovery_engine_core::document::Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .get_feed_documents(max_documents)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes time spent.
    pub async fn time_spent(
        engine: &crate::types::engine::SharedEngine,
        time_spent: Box<xayn_discovery_engine_core::document::TimeSpent>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .time_spent(time_spent.as_ref())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes user reaction.
    pub async fn user_reacted(
        engine: &crate::types::engine::SharedEngine,
        reacted: Box<xayn_discovery_engine_core::document::UserReacted>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .user_reacted(reacted.as_ref())
                .await
                .map_err(|error| error.to_string()),
        )
    }
}
