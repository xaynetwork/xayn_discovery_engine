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

//! Modules containing FFI glue for `Vec<T>`.

use super::{primitives::FfiUsize, slice::boxed_slice_from_raw_parts};

/// Get length of a `Box<Vec<T>>`.
///
/// Be aware that the returned length is clamped to `FfiUnsignedSize::MAX` elements.
pub(super) unsafe fn get_vec_len<T>(vec: *mut Vec<T>) -> FfiUsize {
    FfiUsize::from_usize_lossy(unsafe { &*vec }.len())
}

/// Get a pointer to the beginning of a `Box<Vec<T>>`'s buffer.
pub(super) unsafe fn get_vec_buffer<T>(vec: *mut Vec<T>) -> *mut T {
    unsafe { &mut *vec }.as_mut_ptr()
}

/// Allocates a `Box<Vec<T>>` moving given boxed slice into it.
///
/// # Safety
///
/// - Constructing a `Box<[T]>` from given `slice_ptr`,`slice_len` must be sound.
pub(super) unsafe fn alloc_vec<T>(slice_ptr: *mut T, slice_len: FfiUsize) -> *mut Vec<T> {
    let boxed_slice = unsafe { boxed_slice_from_raw_parts(slice_ptr, slice_len) };
    Box::into_raw(Box::new(Vec::from(boxed_slice)))
}
