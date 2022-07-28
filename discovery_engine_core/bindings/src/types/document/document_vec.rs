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

//! FFI functions for handling slices and vectors of documents.

use xayn_discovery_engine_core::document::Document;

use crate::types::{
    primitives::FfiUsize,
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

/// Initializes a `Vec<Document>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[Document]>`.
///
/// # Safety
///
/// - It must be valid to write a `Vec<Document>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[Document]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_document_vec_at(
    place: *mut Vec<Document>,
    slice_ptr: *mut Document,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[Document]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_document_slice(len: FfiUsize) -> *mut Document {
    alloc_uninitialized_slice(len)
}

/// Given a pointer to a [`Document`] in a slice return the pointer to the next [`Document`].
///
/// This also works for uninitialized document slices.
///
/// # Safety
///
/// The pointer must point to a valid `Document` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_document(place: *mut Document) -> *mut Document {
    unsafe { next_element(place) }
}

/// Drop a `Box<[Document]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[Document]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_document_slice(documents: *mut Document, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(documents, len) });
}

/// Returns the length of a `Box<Vec<Document>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Document>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_document_vec_len(documents: *mut Vec<Document>) -> FfiUsize {
    unsafe { get_vec_len(documents) }
}

/// Returns the `*mut Document` to the beginning of the buffer of a `Box<Vec<Document>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<Document>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_document_vec_buffer(documents: *mut Vec<Document>) -> *mut Document {
    unsafe { get_vec_buffer(documents) }
}

/// Drop a `Box<Vec<Document>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<Document>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_document_vec(documents: *mut Vec<Document>) {
    unsafe {
        crate::types::boxed::drop(documents);
    }
}
