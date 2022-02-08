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

//! FFI functions for handling `String`

use std::{ptr, slice, str};

/// Helper to create a `&str`.
///
/// # Safety
///
/// - The bytes `str_start..str_start+str_len` must be a sound rust string for the
///   lifetime of `'a`.
pub(super) unsafe fn str_from_raw_parts<'a>(str_start: *const u8, str_len: usize) -> &'a str {
    unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(str_start, str_len))
    }
}

/// Creates a rust `String` from given `Box<str>`.
///
/// # Safety
///
/// - It must be valid to write a `String` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - The bytes `str_start..str_start+str_len` must be a sound rust string.
#[no_mangle]
pub unsafe extern "C" fn init_string_at(place: *mut String, str_start: *mut u8, str_len: usize) {
    unsafe {
        ptr::write(place, String::from_raw_parts(str_start, str_len, str_len));
    }
}

/// Returns the length of a rust `String` at given memory address.
///
/// # Safety
///
/// The pointer must point to a valid `String` instance.
#[no_mangle]
pub unsafe extern "C" fn get_string_len(string: *mut String) -> usize {
    unsafe { &*string }.len()
}

/// Returns a pointer to the underlying buffer of the given rust string.
///
/// You can write valid utf8 to the buffer up to a length of the strings
/// `capacity`, after which you can use `set_string_len` to make the written
/// data available to rust.
///
/// # Safety
///
/// The pointer must point to a valid `String` instance.
#[no_mangle]
pub unsafe extern "C" fn get_string_buffer(string: *mut String) -> *mut u8 {
    unsafe { &mut *string }.as_mut_ptr()
}

/// Alloc an uninitialized `Box<String>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_string() -> *mut String {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<String>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_string(boxed: *mut String) {
    use super::boxed::drop;

    unsafe { drop(boxed) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::slice;

    #[test]
    fn test_writing_string_works() {
        let string = "lzkljwejguojgheujkhgj";
        let place = &mut String::new();
        unsafe {
            let boxed = string.to_owned().into_boxed_str();
            let str_len = boxed.len();
            let str_start = Box::into_raw(boxed).cast::<u8>();
            init_string_at(place, str_start, str_len);
        }
        assert_eq!(string, *place);
    }

    #[test]
    fn test_reading_string_works() {
        let mut string = "lzkljwejguojgheujkhgj".to_owned();
        let bytes = unsafe {
            let ptr = (&mut string as *mut String).cast();
            let len = get_string_len(ptr);
            let data_ptr = get_string_buffer(ptr);
            slice::from_raw_parts(data_ptr, len).to_owned()
        };
        let res = String::from_utf8(bytes).unwrap();
        assert_eq!(string, res);
    }
}
