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

use xayn_discovery_engine_core::document::{Document, TrendingTopic};

use super::{boxed, engine::SharedEngine, search::Search};

/// Returns a pointer to the `Result::Ok` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<O, E>` instance.
unsafe fn get_result_ok<O, E>(res: *mut Result<O, E>) -> *mut O {
    match unsafe { &mut *res } {
        Ok(value) => value,
        Err(_) => ptr::null_mut(),
    }
}

/// Returns a pointer to the `Result::Err` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<O, E>` instance.
unsafe fn get_result_err<O, E>(res: *mut Result<O, E>) -> *mut E {
    match unsafe { &mut *res } {
        Ok(_) => ptr::null_mut(),
        Err(err) => err,
    }
}

/// Returns a pointer to the moved `Result::Ok` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<O, E>>` instance.
unsafe fn move_result_ok<O, E>(res: *mut Result<O, E>) -> *mut O {
    match *unsafe { Box::from_raw(res) } {
        Ok(value) => Box::into_raw(Box::new(value)),
        Err(_) => ptr::null_mut(),
    }
}

/// Returns a pointer to the `Vec<u8>` success value or a nullptr.
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

/// Returns a pointer to the `String` error value or a nullptr.
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
/// - The pointer must represent a valid `Box<Result<Vec<u8>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_u8_string(res: *mut Result<Vec<u8>, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Document` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Document, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_document_string_ok(
    res: *mut Result<Document, String>,
) -> *mut Document {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Document, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_document_string_err(
    res: *mut Result<Document, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<Document, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<Document, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_document_string(res: *mut Result<Document, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Vec<Document>` success value or a nullptr.
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

/// Returns a pointer to the `String` error value or a nullptr.
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

/// Drops a `Box<Result<Vec<Document>, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<Vec<Document>, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_document_string(res: *mut Result<Vec<Document>, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Vec<TrendingTopic>` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<TrendingTopic>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_trending_topic_string_ok(
    res: *mut Result<Vec<TrendingTopic>, String>,
) -> *mut Vec<TrendingTopic> {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<TrendingTopic>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_trending_topic_string_err(
    res: *mut Result<Vec<TrendingTopic>, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<Vec<TrendingTopic>, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<Vec<TrendingTopic>, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_trending_topic_string(
    res: *mut Result<Vec<TrendingTopic>, String>,
) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `()` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<(), String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_void_string_ok(res: *mut Result<(), String>) -> *mut () {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<()>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_void_string_err(res: *mut Result<(), String>) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<(), String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<(), String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_void_string(res: *mut Result<(), String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `SharedEngine` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<SharedEngine, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_shared_engine_string_ok(
    res: *mut Result<SharedEngine, String>,
) -> *mut SharedEngine {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<SharedEngine>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_shared_engine_string_err(
    res: *mut Result<SharedEngine, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<SharedEngine, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<SharedEngine, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_shared_engine_string(res: *mut Result<SharedEngine, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the moved `SharedEngine` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<SharedEngine, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn move_result_shared_engine_string_ok(
    res: *mut Result<SharedEngine, String>,
) -> *mut SharedEngine {
    unsafe { move_result_ok(res) }
}

/// Returns a pointer to the `Search` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Search, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_search_string_ok(
    res: *mut Result<Search, String>,
) -> *mut Search {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Search, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_search_string_err(
    res: *mut Result<Search, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<Search, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<Search, String>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_search_string(res: *mut Result<Search, String>) {
    unsafe { boxed::drop(res) }
}

/// Returns a pointer to the `Vec<String>` success value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<String>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_string_string_ok(
    res: *mut Result<Vec<String>, String>,
) -> *mut Vec<String> {
    unsafe { get_result_ok(res) }
}

/// Returns a pointer to the `String` error value or a nullptr.
///
/// # Safety
///
/// - The pointer must point to a sound `Result<Vec<String>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_result_vec_string_string_err(
    res: *mut Result<Vec<String>, String>,
) -> *mut String {
    unsafe { get_result_err(res) }
}

/// Drops a `Box<Result<Vec<String>, String>>`.
///
/// # Safety
///
/// - The pointer must represent a valid `Box<Result<Vec<String>, String>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_result_vec_string_string(res: *mut Result<Vec<String>, String>) {
    unsafe { boxed::drop(res) }
}
