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

//! FFI functions for handling `Box`

use std::mem::MaybeUninit;

/// Allocates a box of a uninitialized `T` and returns the pointer to it.
///
/// Mostly used for testing `init_T_at` ffi functions
#[cfg(feature = "additional-ffi-methods")]
pub(super) fn alloc_uninitialized_box<T>() -> *mut T {
    let uninit = Box::<MaybeUninit<T>>::new(MaybeUninit::uninit());
    Box::into_raw(uninit).cast()
}

/// Drops a `Box<T>`, `T` must be initialized.
///
/// Mostly used for testing `init_T_at` ffi functions
///
/// # Safety
///
/// The pointer must represent a valid `Box<T>` instance.
#[cfg(feature = "additional-ffi-methods")]
pub(super) unsafe fn drop_box<T: ?Sized>(boxed: *mut T) {
    unsafe { Box::from_raw(boxed) };
}
