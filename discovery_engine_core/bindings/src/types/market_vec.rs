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

//! FFI functions for handling slices and vectors of [`Market`] instance.

use xayn_discovery_engine_core::Market;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

use super::boxed::{self, alloc_uninitialized};

/// Initializes a `Vec<Market>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, len` map to `Box<[Market]>`.
///
/// # Safety
///
/// - It must be valid to write an `Option<Vec<Market>>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[Market]>` from given `slice_ptr`
///   and `len`.
#[no_mangle]
pub unsafe extern "C" fn init_market_vec_at(
    place: *mut Vec<Market>,
    slice_ptr: *mut Market,
    len: usize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, len)));
    }
}

/// Alloc an uninitialized `Box<[Market]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_market_slice(len: usize) -> *mut Market {
    alloc_uninitialized_slice(len)
}

/// Drop a `Box<[Market]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[Market]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_market_slice(markets: *mut Market, len: usize) {
    drop(unsafe { boxed_slice_from_raw_parts(markets, len) });
}

/// Alloc an uninitialized `Box<Vec<Market>>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_market_vec() -> *mut Vec<Market> {
    alloc_uninitialized()
}

/// Drop a `Box<Vec<Market>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<Market>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_market_vec(markets: *mut Vec<Market>) {
    unsafe {
        boxed::drop(markets);
    }
}

/// Given a pointer to a [`Market`] in a slice return the pointer to the next [`Market`].
///
/// This also works if the slice is uninitialized.
///
/// # Safety
///
/// The pointer must point to a valid `Market` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_market(place: *mut Market) -> *mut Market {
    unsafe { next_element(place) }
}

/// Returns the length of a `Box<Vec<Market>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Market>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_market_vec_len(markets: *mut Vec<Market>) -> usize {
    unsafe { get_vec_len(markets) }
}

/// Returns the `*mut Market` to the beginning of the buffer of a `Box<Vec<Market>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Market>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_market_vec_buffer(markets: *mut Vec<Market>) -> *mut Market {
    unsafe { get_vec_buffer(markets) }
}
