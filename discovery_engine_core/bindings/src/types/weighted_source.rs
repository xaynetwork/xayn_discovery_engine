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

//! FFI functions for handling weighted sources.

use std::ptr::addr_of_mut;

use xayn_discovery_engine_core::document::WeightedSource;

/// Returns a pointer to the `source` field of a [`WeightedSource`].
///
/// # Safety
///
/// The pointer must point to a valid [`WeightedSource`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn weighted_source_place_of_source(
    place: *mut WeightedSource,
) -> *mut String {
    unsafe { addr_of_mut!((*place).source) }
}

/// Returns a pointer to the `weight` field of a [`WeightedSource`].
///
/// # Safety
///
/// The pointer must point to a valid [`WeightedSource`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn weighted_source_place_of_weight(place: *mut WeightedSource) -> *mut i32 {
    unsafe { addr_of_mut!((*place).weight) }
}

/// Alloc an uninitialized `Box<WeightedSource>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_weighted_source() -> *mut WeightedSource {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<WeightedSource>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<WeightedSource>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_weighted_source(source: *mut WeightedSource) {
    unsafe { crate::types::boxed::drop(source) };
}
