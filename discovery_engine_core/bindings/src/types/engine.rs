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

use cfg_if::cfg_if;
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
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn new(engine: Engine, init_db_hint: InitDbHint) -> InitializationResult {
        let shared_engine = tokio::sync::Mutex::new(engine).into();
        cfg_if! {
            if #[cfg(feature = "storage")] {
                // for now we will only expose the override error converted to an string
                let db_override_error = if let InitDbHint::DbOverwrittenDueToErrors(error) = init_db_hint {
                    Some(error.to_string())
                } else {
                    None
                };
            } else {
                let _ = init_db_hint;
                let db_override_error = None;
            }
        }

        InitializationResult {
            shared_engine,
            db_override_error,
        }
    }
}

/// Returns a pointer to the `db_override_error` field of a [`InitializationResult`].
///
/// # Safety
///
/// The pointer must point to a valid [`InitializationResult`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn initialization_result_place_of_db_override_error(
    init_result: *mut InitializationResult,
) -> *mut Option<String> {
    unsafe { addr_of_mut!((*init_result).db_override_error) }
}

/// Converts a `Box<InitializationResult>` into a `Box<SharedEngine>`.
///
/// From dart the interface would converting `Pointer<RustInitializationResult>`
/// to an `Pointer<RustSharedEngine>` BUT the input pointer must still be that
/// of an rust `Box<InitializationResult>`!
#[no_mangle]
pub extern "C" fn destruct_initialization_result_into_shared_engine(
    init_result: Box<InitializationResult>,
) -> Box<SharedEngine> {
    let InitializationResult {
        shared_engine: engine,
        db_override_error: _,
    } = *init_result;
    Box::new(engine)
}
