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

//! FFI functions for handling `Option<T>`

use std::{ptr, mem::MaybeUninit, hint::unreachable_unchecked};

/// Writes `Some(MaybeUninit::uninit())` to given place.
///
/// # Safety
///
/// - It must be valid to write a `Option<T>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - Before using the option the value must be initialized using the returned pointer.
///   (except if `T` is valid without initialization, e.g. `MaybeUninit`).
pub(super) unsafe fn init_some_at<T>(place: *mut Option<T>) -> *mut T{
    let place = place.cast::<Option<MaybeUninit<T>>>();
    unsafe {
        ptr::write(place, Some(MaybeUninit::uninit()));
    }
    //SAFE as ptr is `Option<MaybeUninit<T>>`
    match unsafe { &mut *place } {
        Some(uninit) => uninit.as_mut_ptr(),
        None => unsafe { unreachable_unchecked() },
    }
}

/// Writes `None` to given place.
///
/// # Safety
///
/// - It must be valid to write a `Option<T>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
pub(super) unsafe fn init_none_at<T>(place: *mut Option<T>) {
    unsafe {
        ptr::write(place, None);
    }
}

/// Returns a pointer to the value in the `Some` variant.
///
/// **Returns nullptr if the `Option<T>` is `None`.**
///
/// This function MUST NOT be used to init a partially initialized option.
///
/// # Safety
///
/// - The pointer must point to a sound initialized `Option<T>`.
pub(super) unsafe fn get_option_some<T>(opt_url: *const Option<T>) -> *const T {
    match unsafe { &*opt_url } {
        Some(val) => val,
        None => ptr::null(),
    }
}
