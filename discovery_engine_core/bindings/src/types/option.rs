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

use std::ptr;

/// Returns a pointer to the value in the `Some` variant.
///
/// **Returns nullptr if the `Option<T>` is `None`.**
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
