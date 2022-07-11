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

//! FFI functions for handling [`Market`] instances.

use std::ptr::addr_of_mut;

use xayn_discovery_engine_providers::Market;

/// Returns a pointer to the `lang_code` field of a [`Market`].
///
/// # Safety
///
/// The pointer must point to a valid [`Market`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn market_place_of_lang_code(place: *mut Market) -> *mut String {
    unsafe { addr_of_mut!((*place).lang_code) }
}

/// Returns a pointer to the `country_code` field of a [`Market`].
///
/// # Safety
///
/// The pointer must point to a valid [`Market`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn market_place_of_country_code(place: *mut Market) -> *mut String {
    unsafe { addr_of_mut!((*place).country_code) }
}

/// Alloc an uninitialized `Box<Market>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_market() -> *mut Market {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<Market>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Market>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_market(reaction: *mut Market) {
    unsafe { crate::types::boxed::drop(reaction) };
}
