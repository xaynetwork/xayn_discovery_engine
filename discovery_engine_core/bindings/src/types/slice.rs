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

//! Modules containing FFI glue for `Box<[u8]>`.

use std::{mem::MaybeUninit, slice};

/// Allocates a slice of (uninitialized) bytes of given length.
pub(super) fn alloc_uninitialized_slice<T>(len: usize) -> *mut T {
    let mut vec = Vec::<MaybeUninit<T>>::with_capacity(len);
    //SAFE: MaybeUninit doesn't need initialization
    unsafe {
        vec.set_len(len);
    }
    let boxed_slice = vec.into_boxed_slice();
    Box::into_raw(boxed_slice).cast()
}

/// Creates a `Box<[T]>` from a pointer to the first element and the slice len.
pub(super) unsafe fn boxed_slice<T>(ptr: *mut T, len: usize) -> Box<[T]> {
    unsafe { Box::from_raw(slice::from_raw_parts_mut(ptr, len)) }
}

/// Get length of an `Box<Vec<T>>`.
#[allow(dead_code)]
pub(super) unsafe fn get_boxed_vec_len<T>(ptr: *mut Vec<T>) -> usize {
    unsafe { &*ptr }.len()
}

/// Get a pointer to the beginning of an `Box<Vec<T>>`
#[allow(dead_code)]
pub(super) unsafe fn get_boxed_vec_buffer<T>(ptr: *mut Vec<T>) -> *mut T {
    unsafe { &mut *ptr }.as_mut_ptr()
}

/// Increments the pointer by one element
///
/// # Safety
///
/// This is basically an alias for `ptr.offset(1)` and
/// all safety constraints from `offset` apply.
#[allow(dead_code)]
pub(super) unsafe fn next_slice_element<T>(ptr: *mut T) -> *mut T {
    unsafe { ptr.offset(1) }
}

/// Allocates an uninitialized slice of bytes
#[no_mangle]
pub extern "C" fn alloc_uninitialized_f32_slice(len: usize) -> *mut f32 {
    alloc_uninitialized_slice(len)
}

/// Drops a `Box<[u8]>`
///
/// # Safety
///
/// The pointer must represent a valid `Box<[f32]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_f32_slice(ptr: *mut f32, len: usize) {
    drop(unsafe { boxed_slice(ptr, len) });
}
