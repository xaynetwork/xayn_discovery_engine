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

use core::document::Document;

use crate::types::{
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

/// Alloc an uninitialized `Box<[Document]>`, i.e. a `Box<[MaybeUninit<Document>]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_document_slice(len: usize) -> *mut Document {
    alloc_uninitialized_slice(len)
}

/// Given a pointer to a [`Document`] in a slice return the pointer to the next [`Document`].
///
/// This also works for a `Box<[MaybeUninit<Document>]>`, i.e. a boxed slice with
/// uninitialized documents.
///
/// # Safety
///
/// The pointer must point to a valid `RustDocument` memory object, it might
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
pub unsafe extern "C" fn drop_document_slice(documents: *mut Document, len: usize) {
    drop(unsafe { boxed_slice_from_raw_parts(documents, len) });
}

/// Returns the length of a `Box<Vec<T>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<T>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_document_vec_len(documents: *mut Vec<Document>) -> usize {
    unsafe { get_vec_len(documents) }
}

/// Returns the `*mut T` to the beginning of the buffer of a `Box<Vec<T>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<T>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_document_vec_buffer(documents: *mut Vec<Document>) -> *mut Document {
    unsafe { get_vec_buffer(documents) }
}

/// Drop a `Box<Vec<T>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<T>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_document_vec(documents: *mut Vec<Document>) {
    unsafe {
        crate::types::boxed::drop(documents);
    }
}
