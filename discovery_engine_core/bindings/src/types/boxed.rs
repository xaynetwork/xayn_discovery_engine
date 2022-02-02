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

//! FFI functions for handling `Box<T>`.

use std::mem::MaybeUninit;

/// Allocates a box of an uninitialized `T` and returns the pointer to it.
///
/// Mostly used for testing `init_T_at` ffi functions
pub(super) fn alloc_uninitialized<T>() -> *mut T {
    Box::into_raw(Box::new(MaybeUninit::<T>::uninit())).cast()
}

/// Drops a `Box<T>`, `T` must be initialized.
///
/// Mostly used for testing `init_T_at` ffi functions
///
/// # Safety
///
/// The pointer must represent a valid `Box<T>` instance.
pub(super) unsafe fn drop<T: ?Sized>(boxed: *mut T) {
    unsafe { Box::from_raw(boxed) };
}
