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

//! Modules containing FFI glue for arrays (i.e. `Box<[T]>`) handling.

use std::{mem::MaybeUninit, slice};

/// Allocates an array of (uninitialized) `T` memory objects of given length.
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
pub(super) unsafe fn boxed_slice_from_raw_parts<T>(ptr: *mut T, len: usize) -> Box<[T]> {
    unsafe { Box::from_raw(slice::from_raw_parts_mut(ptr, len)) }
}

/// Given a pointer to an element in an array, returns a pointer to the next element.
///
/// # Safety
///
/// This is basically an alias for `ptr.offset(1)` and
/// all safety constraints from `offset` apply.
#[allow(dead_code)]
pub(super) unsafe fn next_element<T>(element: *mut T) -> *mut T {
    unsafe { element.offset(1) }
}

/// Allocates an uninitialized array of floats.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_f32_slice(len: usize) -> *mut f32 {
    alloc_uninitialized_slice(len)
}

/// Drops a `Box<[f32]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[f32]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_f32_slice(ptr: *mut f32, len: usize) {
    drop(unsafe { boxed_slice_from_raw_parts(ptr, len) });
}
