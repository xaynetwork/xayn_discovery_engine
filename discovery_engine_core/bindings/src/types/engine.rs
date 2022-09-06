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

//! FFI functions for handling engine instances.

use std::ptr::addr_of_mut;

use derive_more::{AsRef, From};
use tokio::sync::Mutex;

use xayn_discovery_engine_core::{Engine, InitDbHint};

/// A shared discovery engine with a lock.
#[derive(AsRef, From)]
pub struct SharedEngine(Mutex<Engine>);

pub struct InitializationResult {
    pub shared_engine: SharedEngine,
    pub db_override_error: Option<String>,
}

impl InitializationResult {
    pub(crate) fn new(engine: Engine, init_db_hint: InitDbHint) -> InitializationResult {
        let shared_engine = tokio::sync::Mutex::new(engine).into();
        // for now we will only expose the override error converted to an string
        let db_override_error = if let InitDbHint::DbOverwrittenDueToErrors(error) = init_db_hint {
            Some(error.to_string())
        } else {
            None
        };

        InitializationResult {
            shared_engine,
            db_override_error,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn initialization_result_place_of_db_override_error(
    init_result: *mut InitializationResult,
) -> *mut Option<String> {
    unsafe { addr_of_mut!((*init_result).db_override_error) }
}

#[no_mangle]
pub unsafe extern "C" fn destruct_initialization_result_into_shared_engine(
    init_result: Box<InitializationResult>,
) -> Box<SharedEngine> {
    let InitializationResult {
        shared_engine: engine,
        db_override_error: _,
    } = *init_result;
    Box::new(engine)
}
