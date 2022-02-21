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

//! FFI functions for handling slices and vectors of [`HistoricDocument`]s.

use xayn_discovery_engine_core::document::HistoricDocument;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

use super::boxed::alloc_uninitialized;

/// Initializes a `Vec<HistoricDocument>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, len` map to `Box<[HistoricDocument]>`.
///
/// # Safety
///
/// - It must be valid to write a `Vec<HistoricDocument>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[HistoricDocument]>` from given `slice_ptr`
///   and `len`.
#[no_mangle]
pub unsafe extern "C" fn init_historic_document_vec_at(
    place: *mut Vec<HistoricDocument>,
    slice_ptr: *mut HistoricDocument,
    slice_len: usize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[HistoricDocument]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_historic_document_slice(len: usize) -> *mut HistoricDocument {
    alloc_uninitialized_slice(len)
}

/// Alloc an uninitialized `Box<Vec<HistoricDocument>>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_historic_document_vec() -> *mut Vec<HistoricDocument> {
    alloc_uninitialized()
}

/// Given a pointer to a [`HistoricDocument`] in a slice return the pointer to the next [`HistoricDocument`].
///
/// This also works for uninitialized historic document slices.
///
/// # Safety
///
/// The pointer must point to a valid `HistoricDocument` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_historic_document(
    place: *mut HistoricDocument,
) -> *mut HistoricDocument {
    unsafe { next_element(place) }
}

/// Drop a `Box<[HistoricDocument]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[HistoricDocument]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_historic_document_slice(
    documents: *mut HistoricDocument,
    len: usize,
) {
    drop(unsafe { boxed_slice_from_raw_parts(documents, len) });
}

/// Returns the length of a `Box<Vec<HistoricDocument>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<HistoricDocument>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_historic_document_vec_len(
    documents: *mut Vec<HistoricDocument>,
) -> usize {
    unsafe { get_vec_len(documents) }
}

/// Returns the `*mut HistoricDocument` to the beginning of the buffer of a `Box<Vec<HistoricDocument>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<HistoricDocument>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_historic_document_vec_buffer(
    documents: *mut Vec<HistoricDocument>,
) -> *mut HistoricDocument {
    unsafe { get_vec_buffer(documents) }
}

/// Drop a `Box<Vec<HistoricDocument>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<HistoricDocument>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_historic_document_vec(documents: *mut Vec<HistoricDocument>) {
    unsafe {
        crate::types::boxed::drop(documents);
    }
}
