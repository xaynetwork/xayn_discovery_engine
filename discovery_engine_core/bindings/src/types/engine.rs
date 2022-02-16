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

use derive_more::{AsRef, From};
use tokio::sync::Mutex;

use xayn_discovery_engine_core::XaynAiEngine;

use super::boxed;

/// A shared discovery engine with a lock.
#[derive(AsRef, From)]
pub struct SharedEngine(Mutex<XaynAiEngine>);

/// Drops a `Box<SharedEngine>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<SharedEngine>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_shared_engine(engine: *mut SharedEngine) {
    unsafe { boxed::drop(engine) }
}
