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

//! FFI functions for handling slices and vectors of [`Uuid`] instance.

use uuid::Uuid;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

use super::{
    boxed::{self, alloc_uninitialized},
    primitives::FfiUsize,
};

/// Initializes a `Vec<Uuid>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[Uuid]>`.
///
/// # Safety
///
/// - It must be valid to write an `Vec<Uuid>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[Uuid]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_uuid_vec_at(
    place: *mut Vec<Uuid>,
    slice_ptr: *mut Uuid,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[Uuid]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_uuid_slice(len: FfiUsize) -> *mut Uuid {
    alloc_uninitialized_slice(len)
}

/// Drop a `Box<[Uuid]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[Uuid]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_uuid_slice(ids: *mut Uuid, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(ids, len) });
}

/// Alloc an uninitialized `Box<Vec<Uuid>>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_uuid_vec() -> *mut Vec<Uuid> {
    alloc_uninitialized()
}

/// Drop a `Box<Vec<Uuid>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<Uuid>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_uuid_vec(ids: *mut Vec<Uuid>) {
    unsafe {
        boxed::drop(ids);
    }
}

/// Given a pointer to a [`Uuid`] in a slice return the pointer to the next [`Uuid`].
///
/// This also works if the slice is uninitialized.
///
/// # Safety
///
/// The pointer must point to a valid `Uuid` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_uuid(place: *mut Uuid) -> *mut Uuid {
    unsafe { next_element(place) }
}

/// Returns the length of a `Box<Vec<Uuid>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Uuid>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_uuid_vec_len(ids: *mut Vec<Uuid>) -> FfiUsize {
    unsafe { get_vec_len(ids) }
}

/// Returns the `*mut Uuid` to the beginning of the buffer of a `Box<Vec<Uuid>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Uuid>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_uuid_vec_buffer(ids: *mut Vec<Uuid>) -> *mut Uuid {
    unsafe { get_vec_buffer(ids) }
}
