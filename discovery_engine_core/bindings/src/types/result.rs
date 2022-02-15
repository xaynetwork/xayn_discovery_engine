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

//! Modules containing FFI glue for handling `Result` instances.

use std::ptr;

use core::document::Document;

use super::boxed;

/// Returns a pointer to the `Result::Ok` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<O, E>` instance.
pub(super) unsafe fn get_result_ok<O, E>(res: *mut Result<O, E>) -> *mut O {
    match unsafe { &mut *res } {
        Ok(value) => value,
        Err(_) => ptr::null_mut(),
    }
}

/// Returns a pointer to the `Result::Err` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<O, E>` instance.
pub(super) unsafe fn get_result_err<O, E>(res: *mut Result<O, E>) -> *mut E {
    match unsafe { &mut *res } {
        Ok(_) => ptr::null_mut(),
        Err(err) => err,
    }
}

/// Returns a pointer to the `Result::Ok` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<u8>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_u8_string_ok(
    res: *mut Result<Vec<u8>, String>,
) -> *mut Vec<u8> {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `Result::Err` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<u8>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_u8_string_err(
    res: *mut Result<Vec<u8>, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<Vec<u8>, String>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Result<Vec<u8>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_u8_string(res: *mut Box<Result<Vec<u8>, String>>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Result::Ok` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<Document>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_document_string_ok(
    res: *mut Result<Vec<Document>, String>,
) -> *mut Vec<Document> {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `Result::Err` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<Document>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_document_string_err(
    res: *mut Result<Vec<Document>, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Result<Vec<Document>, String>`.
///
/// # Safety
///
/// The pointer must represent a valid `Result<Vec<Document>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_document_string(res: *mut Result<Vec<Document>, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Result::Ok` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<(), String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_void_string_ok(res: *mut Result<(), String>) -> *mut () {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `Result::Err` value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<()>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_void_string_err(res: *mut Result<(), String>) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Result<(), String>`.
///
/// # Safety
///
/// The pointer must represent a valid `Result<(), String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_void_string(res: *mut Result<(), String>) {
    unsafe { boxed::drop(res) }
}
