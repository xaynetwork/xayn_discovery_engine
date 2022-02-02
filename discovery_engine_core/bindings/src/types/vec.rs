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

/// Get length of a `Box<Vec<T>>`.
#[allow(dead_code)]
pub(super) unsafe fn get_vec_len<T>(vec: *mut Vec<T>) -> usize {
    unsafe { &*vec }.len()
}

/// Get a pointer to the beginning of a `Box<Vec<T>>`'s buffer.
#[allow(dead_code)]
pub(super) unsafe fn get_vec_buffer<T>(vec: *mut Vec<T>) -> *mut T {
    unsafe { &mut *vec }.as_mut_ptr()
}
