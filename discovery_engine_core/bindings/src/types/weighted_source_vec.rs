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

//! FFI functions for handling slices and vectors of [`WeightedSource`]s.

use xayn_discovery_engine_core::document::WeightedSource;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

use super::{boxed::alloc_uninitialized, primitives::FfiUsize};

/// Initializes a `Vec<WeightedSource>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[WeightedSource]>`.
///
/// # Safety
///
/// - It must be valid to write a `Vec<WeightedSource>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[WeightedSource]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_weighted_source_vec_at(
    place: *mut Vec<WeightedSource>,
    slice_ptr: *mut WeightedSource,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[WeightedSource]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_weighted_source_slice(len: FfiUsize) -> *mut WeightedSource {
    alloc_uninitialized_slice(len)
}

/// Alloc an uninitialized `Box<Vec<WeightedSource>>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_weighted_source_vec() -> *mut Vec<WeightedSource> {
    alloc_uninitialized()
}

/// Given a pointer to a [`WeightedSource`] in a slice return the pointer to the next [`WeightedSource`].
///
/// This also works for uninitialized weighted source slices.
///
/// # Safety
///
/// The pointer must point to a valid `WeightedSource` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_weighted_source(place: *mut WeightedSource) -> *mut WeightedSource {
    unsafe { next_element(place) }
}

/// Drop a `Box<[WeightedSource]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[WeightedSource]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_weighted_source_slice(sources: *mut WeightedSource, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(sources, len) });
}

/// Returns the length of a `Box<Vec<WeightedSource>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<WeightedSource>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_weighted_source_vec_len(
    sources: *mut Vec<WeightedSource>,
) -> FfiUsize {
    unsafe { get_vec_len(sources) }
}

/// Returns the `*mut WeightedSource` to the beginning of the buffer of a `Box<Vec<WeightedSource>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<WeightedSource>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_weighted_source_vec_buffer(
    sources: *mut Vec<WeightedSource>,
) -> *mut WeightedSource {
    unsafe { get_vec_buffer(sources) }
}

/// Drop a `Box<Vec<WeightedSource>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<WeightedSource>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_weighted_source_vec(sources: *mut Vec<WeightedSource>) {
    unsafe {
        crate::types::boxed::drop(sources);
    }
}
