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

//! FFI functions for handling slices and vectors of [`String`] instance.

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

use super::{
    boxed::{self, alloc_uninitialized},
    primitives::FfiUsize,
};

/// Initializes a `Vec<String>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[String]>`.
///
/// # Safety
///
/// - It must be valid to write an `Option<Vec<String>>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[String]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_string_vec_at(
    place: *mut Vec<String>,
    slice_ptr: *mut String,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[String]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_string_slice(len: FfiUsize) -> *mut String {
    alloc_uninitialized_slice(len)
}

/// Drop a `Box<[String]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[String]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_string_slice(strings: *mut String, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(strings, len) });
}

/// Alloc an uninitialized `Box<Vec<String>>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_string_vec() -> *mut Vec<String> {
    alloc_uninitialized()
}

/// Drop a `Box<Vec<String>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_string_vec(strings: *mut Vec<String>) {
    unsafe {
        boxed::drop(strings);
    }
}

/// Given a pointer to a [`String`] in a slice return the pointer to the next [`String`].
///
/// This also works if the slice is uninitialized.
///
/// # Safety
///
/// The pointer must point to a valid `String` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_string(place: *mut String) -> *mut String {
    unsafe { next_element(place) }
}

/// Returns the length of a `Box<Vec<String>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_string_vec_len(strings: *mut Vec<String>) -> FfiUsize {
    unsafe { get_vec_len(strings) }
}

/// Returns the `*mut String` to the beginning of the buffer of a `Box<Vec<String>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_string_vec_buffer(strings: *mut Vec<String>) -> *mut String {
    unsafe { get_vec_buffer(strings) }
}
