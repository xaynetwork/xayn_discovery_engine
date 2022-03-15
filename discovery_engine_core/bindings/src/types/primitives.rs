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

//! Modules containing FFI glue for handling primitives (expect `str`/`slice`).

use std::convert::TryInto;

use super::{
    boxed,
    option::get_option_some,
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts},
    vec::{alloc_vec, get_vec_buffer, get_vec_len},
};

/// Type to represent "length" in FFI.
///
/// Be aware that values returned are bounded
/// by 2**32, e.g. for `Vec` the extern interface will
/// only expose the first 2**32 elements of a
/// `Vec`.
// FIXME[dart >1.16]: Use AbiSpecificInteger.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FfiUsize(u32);

impl FfiUsize {
    /// Returns the value as `usize`.
    pub fn to_usize(self) -> usize {
        #[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
        compile_error!("Only 32/64bit targets are supported");
        self.0 as usize
    }

    /// Creates an instance from an `usize`.
    ///
    /// The value is bounded by `2**32`.
    pub fn from_usize_lossy(v: usize) -> Self {
        Self(v.try_into().unwrap_or(u32::MAX))
    }
}

/// Initializes a rust `Option<f32>` to `Some(value)`.
///
/// # Safety
///
/// - It must be valid to write an `Option<f32>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_some_f32_at(place: *mut Option<f32>, value: f32) {
    unsafe {
        place.write(Some(value));
    }
}

/// Initializes a rust `Option<f32>` to `None`.
///
/// # Safety
///
/// - It must be valid to write an `Option<f32>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_none_f32_at(place: *mut Option<f32>) {
    unsafe {
        place.write(None);
    }
}

/// Returns a pointer to the value in the `Some` variant.
///
/// **Returns nullptr if the `Option<f32>` is `None`.**
///
/// # Safety
///
/// - The pointer must point to a sound initialized `Option<f32>`.
#[no_mangle]
pub unsafe extern "C" fn get_option_f32_some(option: *const Option<f32>) -> *const f32 {
    unsafe { get_option_some(option) }
}

/// Allocates an uninitialized array of floats.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_f32_slice(len: FfiUsize) -> *mut f32 {
    alloc_uninitialized_slice(len)
}

/// Drops a `Box<[f32]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[f32]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_f32_slice(ptr: *mut f32, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(ptr, len) });
}

/// Allocates an uninitialized array of bytes.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_bytes(len: FfiUsize) -> *mut u8 {
    alloc_uninitialized_slice(len)
}

/// Drops a `Box<[u8]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[u8]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_bytes(ptr: *mut u8, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(ptr, len) });
}

/// Allocates a `Box<Vec<u8>>` moving given boxed slice into it.
///
/// # Safety
///
/// - Constructing a `Box<[u8]>` from given `slice_ptr`,`slice_len` must be sound.
#[no_mangle]
pub unsafe extern "C" fn alloc_vec_u8(slice_ptr: *mut u8, slice_len: FfiUsize) -> *mut Vec<u8> {
    unsafe { alloc_vec(slice_ptr, slice_len) }
}

/// Drops a `Box<Vec<u8>>`.
///
/// # Safety
///
/// The pointer must represent a `Box<Vec<u8>>`.
#[no_mangle]
pub unsafe extern "C" fn drop_vec_u8(ptr: *mut Vec<u8>) {
    unsafe { boxed::drop(ptr) }
}

/// Returns the length of a `Box<Vec<u8>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<u8>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_vec_u8_len(vec: *mut Vec<u8>) -> FfiUsize {
    unsafe { get_vec_len(vec) }
}

/// Returns the `*mut u8` to the beginning of the buffer of a `Box<Vec<u8>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<u8>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_vec_u8_buffer(vec: *mut Vec<u8>) -> *mut u8 {
    unsafe { get_vec_buffer(vec) }
}
