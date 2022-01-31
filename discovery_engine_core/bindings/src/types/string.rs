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

use std::ptr;

/// Creates a rust `String` with given capacity at given memory address.
///
/// The `String` is created at given address, not it's content.
///
/// # Safety
///
/// It must be valid to write a `String` instance to given pointer,
/// the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_string_at(place: *mut String, capacity: usize) -> *mut u8 {
    let mut s = String::with_capacity(capacity);
    let data_ptr = s.as_mut_ptr();
    unsafe {
        ptr::write(place, s);
    }
    data_ptr
}

/// Sets the length of the rust `String` at given memory address.
///
/// Use this after you write to the strings data buffer behind
/// `len` (but before it's end indicated by `capacity`) to make
/// the newly written string data available to rust.
///
/// # Safety
///
/// The pointer must point to a valid `String` instance.
#[no_mangle]
pub unsafe extern "C" fn set_string_len(string_place: *mut String, len: usize) {
    unsafe {
        (*string_place).as_mut_vec().set_len(len);
    }
}

/// Returns the length of an rust `String` at given memory address.
///
/// # Safety
///
/// The pointer must point to a valid `String` instance.
#[no_mangle]
pub unsafe extern "C" fn get_string_len(string: *mut String) -> usize {
    unsafe { (*string).len() }
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
#[cfg(feature = "additional-ffi-methods")]
#[no_mangle]
pub extern "C" fn alloc_uninitialized_string_box() -> *mut String {
    use super::boxed::alloc_uninitialized_box;

    alloc_uninitialized_box()
}

/// Drops a `Box<String>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<String>` instance.
#[cfg(feature = "additional-ffi-methods")]
#[no_mangle]
pub unsafe extern "C" fn drop_string_box(boxed: *mut String) {
    use super::boxed::drop_box;

    unsafe { drop_box(boxed) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ptr, slice};

    #[test]
    fn test_writing_string_works() {
        let string = "lzkljwejguojgheujkhgj";
        let dest = &mut String::new();
        unsafe {
            let ptr = (dest as *mut String).cast();
            let len = string.len();
            let data_ptr = init_string_at(ptr, len);
            assert!(dest.is_empty());
            assert!(dest.capacity() >= len);
            ptr::copy(string.as_ptr(), data_ptr, len);
            set_string_len(ptr, len);
        }
        assert_eq!(string, *dest);
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
