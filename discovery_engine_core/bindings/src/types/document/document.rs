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

//! FFI functions for handling documents.

use core::document::{Document, Embedding};
use std::ptr::addr_of_mut;

use uuid::Uuid;

/// Returns a pointer to the `id` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_id(place: *mut Document) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id.0) }
}

/// Returns a pointer to the `stack_id` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_stack_id(place: *mut Document) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).stack_id.0) }
}

/// Returns a pointer to the `rank` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_rank(place: *mut Document) -> *mut usize {
    unsafe { addr_of_mut!((*place).rank) }
}

/// Returns a pointer to the `title` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_title(place: *mut Document) -> *mut String {
    unsafe { addr_of_mut!((*place).title) }
}

/// Returns a pointer to the `snipped` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_snipped(place: *mut Document) -> *mut String {
    unsafe { addr_of_mut!((*place).snippet) }
}

/// Returns a pointer to the `url` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_url(place: *mut Document) -> *mut String {
    unsafe { addr_of_mut!((*place).url) }
}

/// Returns a pointer to the `domain` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_domain(place: *mut Document) -> *mut String {
    unsafe { addr_of_mut!((*place).domain) }
}

/// Returns a pointer to the `smbert_embedding` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_smbert_embedding(
    place: *mut Document,
) -> *mut Embedding {
    unsafe { addr_of_mut!((*place).smbert_embedding) }
}

/// Alloc an uninitialized `Box<Document>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_document() -> *mut Document {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<Document>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Document>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_document(document: *mut Document) {
    unsafe { crate::types::boxed::drop(document) };
}
