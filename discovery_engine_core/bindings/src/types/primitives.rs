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

use super::option::get_option_some;

/// Initializes a rust `Option<f32>` to `Some(value)`.
///
/// # Safety
///
/// - It must be valid to write a `f32` instance to given pointer,
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
/// - It must be valid to write a `f32` instance to given pointer,
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
