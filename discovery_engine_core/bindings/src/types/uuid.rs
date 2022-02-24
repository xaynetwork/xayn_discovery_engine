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

//! FFI functions for handling `Uuid`

use uuid::Uuid;

use super::string::str_from_raw_parts;

/// Parses a string uuid into a rust Uuid and writes it to the given place.
///
/// Return 1 if it succeeded 0 otherwise.
///
/// # Safety
///
/// It must be sound to `ptr::write` a `Uuid` to place.
#[no_mangle]
pub unsafe extern "C" fn init_uuid_from_string_at(place: *mut Uuid, str_ptr: *const u8, str_len: usize) -> u8 {
    let s = unsafe { str_from_raw_parts(str_ptr, str_len) };
    if let Ok(uuid) = Uuid::parse_str(s) {
        unsafe { place.write(uuid); }
        1
    } else {
        0
    }
}

/// Returns a `Box<[u8; 36]>` containing the string formatting of the uuid.
///
/// # Safety
///
/// - `uuid` must point to a sound Uuid instance
#[no_mangle]
pub unsafe extern "C" fn get_uuid_as_string36(uuid: *mut Uuid) -> *mut u8 {
    let boxed = unsafe { *uuid }.to_string().into_boxed_str();
    Box::into_raw(boxed).cast::<u8>()
}

// /// Creates a new UUID based on this bytes (~ `[u8; 16]`).
// ///
// /// The bytes are passed in as separate parameters as dart
// /// can't handle C values on the stack well.
// ///
// /// # Safety
// ///
// /// It must be valid to write an [`Uuid`] to given pointer.
// #[no_mangle]
// pub unsafe extern "C" fn init_uuid_at(
//     place: *mut Uuid,
//     b0: u8,
//     b1: u8,
//     b2: u8,
//     b3: u8,
//     b4: u8,
//     b5: u8,
//     b6: u8,
//     b7: u8,
//     b8: u8,
//     b9: u8,
//     b10: u8,
//     b11: u8,
//     b12: u8,
//     b13: u8,
//     b14: u8,
//     b15: u8,
// ) {
//     let uuid = Uuid::from_bytes([
//         b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15,
//     ]);
//     unsafe { ptr::write(place, uuid) }
// }

// /// Returns a pointer to the beginning of the 16 byte long byte slice.
// ///
// /// # Safety
// ///
// /// The pointer must point to an initialized [`Uuid`].
// #[no_mangle]
// pub unsafe extern "C" fn get_uuid_bytes(uuid: *mut Uuid) -> *const u8 {
//     let uuid = unsafe { &*uuid };
//     uuid.as_bytes().as_ptr()
// }

/// Alloc an uninitialized `Box<Uuid>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_uuid() -> *mut Uuid {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<Uuid>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent an initialized `Box<Uuid>`.
#[no_mangle]
pub unsafe extern "C" fn drop_uuid(uuid: *mut Uuid) {
    unsafe { super::boxed::drop(uuid) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reading_written_works() {
        let place = &mut Uuid::nil();
        let uuid = "db754919-2038-49f9-a4c3-fcbe4f035477";

        let res = unsafe { init_uuid_from_string_at(place, uuid.as_ptr(), uuid.len()) };
        assert_eq!(res, 1);
        let res_bytes = unsafe {
            let ptr = get_uuid_as_string36(place);
            Vec::from_raw_parts(ptr, 36, 36)
        };
        let res = String::from_utf8(res_bytes).unwrap();
        assert_eq!(res, uuid);
    }

    // #[test]
    // fn test_reading_uuid_works() {
    //     let place = &mut Uuid::new_v4();
    //     let read = unsafe {
    //         let data_ptr = get_uuid_bytes(place);
    //         Uuid::from_slice(slice::from_raw_parts(data_ptr, 16)).unwrap()
    //     };
    //     assert_eq!(*place, read);
    // }

    // #[test]
    // fn test_writing_uuid_works() {
    //     let uuid = Uuid::new_v4();
    //     let place = &mut Uuid::nil();
    //     unsafe {
    //         let b = uuid.as_bytes();
    //         init_uuid_at(
    //             place, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11],
    //             b[12], b[13], b[14], b[15],
    //         );
    //     }
    //     assert_eq!(uuid, *place);
    // }

    // #[test]
    // fn test_reading_writing_uuid_works() {
    //     let uuid = Uuid::new_v4();
    //     let place = alloc_uninitialized_uuid();
    //     let b = uuid.as_bytes();
    //     unsafe {
    //         init_uuid_at(
    //             place, b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11],
    //             b[12], b[13], b[14], b[15],
    //         );
    //     }
    //     let got = unsafe {
    //         let ptr = get_uuid_bytes(place);
    //         Uuid::from_slice(slice::from_raw_parts(ptr, 16)).unwrap()
    //     };
    //     assert_eq!(uuid, got);
    // }
}
