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

//! FFI functions for handling `Url`

use std::ptr;

use url::Url;

use super::{string::str_from_raw_parts, option::{init_some_at, init_none_at, get_option_some}};

/// Creates a rust `Url` based on given parsing given `&str` at given place.
///
/// Return `1` if it succeeds `0` otherwise.
///
/// # Safety
///
/// - It must be valid to write a `Url` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - The bytes `str_start..str_start+str_len` must be a sound rust `str`.
#[no_mangle]
pub unsafe extern "C" fn init_url_at(place: *mut Url, str_start: *const u8, str_len: usize) -> u8 {
    let str = unsafe { str_from_raw_parts(str_start, str_len) };
    if let Ok(url) = Url::parse(str) {
        unsafe {
            ptr::write(place, url);
        }
        1
    } else {
        0
    }
}

/// Create a rust `Option<Url>` partially initialized to `Some(MaybeUninit::uninit())`.
///
/// # Safety
///
/// - It must be valid to write a `Option<Url>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - The returned pointer must be used to initialize the `Url` before the
///   place is used anywhere.
#[no_mangle]
pub unsafe extern "C" fn init_some_url_at(place: *mut Option<Url>) -> *mut Url {
    unsafe { init_some_at(place) }
}

/// Creates a `None` variant of `Option<Url>` at given place.
///
///
/// # Safety
///
/// - It must be valid to write a `Option<Url>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn inti_none_url_at(place: *mut Option<Url>) {
    unsafe { init_none_at(place); }
}

/// Returns a pointer to the start of the `str` buffer in an `Url` instance.
///
/// # Safety
///
/// - The pointer must point to a sound initialized `Url` instance.
#[no_mangle]
pub unsafe extern "C" fn get_url_buffer(url: *const Url) -> *const u8 {
    unsafe { &*url }.as_str().as_ptr()
}

/// Returns teh length of the `str` buffer in an `Url` instance.
///
/// # Safety
///
/// - The pointer must point to a sound initialized `Url` instance.
#[no_mangle]
pub unsafe extern "C" fn get_url_buffer_len(url: *const Url) -> usize {
    unsafe { &*url }.as_str().len()
}

/// Returns a pointer to the value in the `Some` variant.
///
/// **Returns nullptr if the `Option<Url>` is `None`.**
///
/// # Safety
///
/// - The pointer must point to a sound initialized `Option<Url>`.
#[no_mangle]
pub unsafe extern "C" fn get_option_url_some(opt_url: *const Option<Url>) -> *const Url {
    unsafe { get_option_some(opt_url) }
}

/// Alloc an uninitialized `Box<Url>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_url() -> *mut Url {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<Url>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent an initialized `Box<Uuid>`.
#[no_mangle]
pub unsafe extern "C" fn drop_url(uuid: *mut Url) {
    unsafe { super::boxed::drop(uuid) }
}

/// Alloc an uninitialized `Box<Option<Url>>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_option_url() -> *mut Option<Url> {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<Option<Url>>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent an initialized `Box<Option<Url>>`.
#[no_mangle]
pub unsafe extern "C" fn drop_option_url(uuid: *mut Option<Url>) {
    unsafe { super::boxed::drop(uuid) }
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use super::*;

    #[test]
    fn test_creating_url() {
        let url = "https://foo.example/bar";
        let place = &mut MaybeUninit::<Url>::uninit();
        unsafe {
            let ok = init_url_at(place.as_mut_ptr(), url.as_ptr(), url.len());
            assert_eq!(ok, 1);
        }
        let place = unsafe { place.assume_init_mut() };
        assert_eq!(*place, Url::parse(url).unwrap());
    }

    #[test]
    fn test_crating_url_fails() {
        let url = "not_an_url";
        let place = &mut MaybeUninit::<Url>::uninit();
        unsafe {
            let ok = init_url_at(place.as_mut_ptr(), url.as_ptr(), url.len());
            assert_eq!(ok, 0);
        }
    }
}
