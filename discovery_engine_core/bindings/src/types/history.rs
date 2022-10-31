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

//! FFI functions for handling [`HistoricDocument`].

use std::ptr::addr_of_mut;

use url::Url;
use uuid::Uuid;
use xayn_discovery_engine_core::document::HistoricDocument;

/// Returns a pointer to the `id` field of a [`HistoricDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`HistoricDocument`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn historic_document_place_of_id(place: *mut HistoricDocument) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast::<Uuid>()
}

/// Returns a pointer to the `url` field of a [`HistoricDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`HistoricDocument`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn historic_document_place_of_url(place: *mut HistoricDocument) -> *mut Url {
    unsafe { addr_of_mut!((*place).url) }
}

/// Returns a pointer to the `snippet` field of a [`HistoricDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`HistoricDocument`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn historic_document_place_of_snippet(
    place: *mut HistoricDocument,
) -> *mut String {
    unsafe { addr_of_mut!((*place).snippet) }
}

/// Returns a pointer to the `title` field of a [`HistoricDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`HistoricDocument`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn historic_document_place_of_title(
    place: *mut HistoricDocument,
) -> *mut String {
    unsafe { addr_of_mut!((*place).title) }
}

/// Alloc an uninitialized `Box<HistoricDocument>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_historic_document() -> *mut HistoricDocument {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<HistoricDocument>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<HistoricDocument>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_historic_document(doc: *mut HistoricDocument) {
    unsafe { crate::types::boxed::drop(doc) };
}
