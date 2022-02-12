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

use core::Market;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

/// Alloc an uninitialized `Box<[Market]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_market_slice(len: usize) -> *mut Market {
    alloc_uninitialized_slice(len)
}

/// Given a pointer to a [`Market`] in a slice return the pointer to the next [`Market`].
///
/// This also works if the slice is uninitialized.
///
/// # Safety
///
/// The pointer must point to a valid `RustMarket` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_market(place: *mut Market) -> *mut Market {
    unsafe { next_element(place) }
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

/// Returns the length of a `Box<Vec<T>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<T>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_market_vec_len(markets: *mut Vec<Market>) -> usize {
    unsafe { get_vec_len(markets) }
}

/// Returns the `*mut T` to the beginning of the buffer of a `Box<Vec<T>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<T>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_market_vec_buffer(markets: *mut Vec<Market>) -> *mut Market {
    unsafe { get_vec_buffer(markets) }
}
