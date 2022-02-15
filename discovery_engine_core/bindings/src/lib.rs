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
    pub async fn init_engine(
        config: Box<core::InitConfig>,
        state: Box<Option<Vec<u8>>>,
        error: &mut String,
    ) -> Box<Option<core::LockedEngine>> {
        use core::{Engine, LockedEngine};
        use tokio::sync::Mutex;

        let engine = match Engine::from_config(*config, state.as_deref()).await {
            Ok(engine) => Some(LockedEngine(Mutex::new(engine))),
            Err(err) => {
                error.clear();
                error.push_str(&err.to_string());
                None
            }
        };

        Box::new(engine)
    }
}
