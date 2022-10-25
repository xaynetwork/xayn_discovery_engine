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

use std::ptr::addr_of_mut;

use uuid::Uuid;

use xayn_discovery_engine_core::{
    document::{Id, NewsResource, UserReaction},
    stack::Id as StackId,
};

pub struct Document {
    id: Id,
    stack_id: StackId,
    reaction: Option<UserReaction>,
    resource: NewsResource,
}

impl From<xayn_discovery_engine_core::document::Document> for Document {
    fn from(document: xayn_discovery_engine_core::document::Document) -> Self {
        Self {
            id: document.id,
            stack_id: document.stack_id,
            reaction: document.reaction,
            resource: document.resource,
        }
    }
}

/// Returns a pointer to the `id` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_id(place: *mut Document) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast::<Uuid>()
}

/// Returns a pointer to the `stack_id` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_stack_id(place: *mut Document) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).stack_id) }.cast::<Uuid>()
}

/// Returns a pointer to the `reaction` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_reaction(
    place: *mut Document,
) -> *mut Option<UserReaction> {
    unsafe { addr_of_mut!((*place).reaction) }
}

/// Returns a pointer to the `resource` field of a document.
///
/// # Safety
///
/// The pointer must point to a valid [`Document`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn document_place_of_resource(place: *mut Document) -> *mut NewsResource {
    unsafe { addr_of_mut!((*place).resource) }
}

/// Alloc an uninitialized `Box<Document>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_document() -> *mut Document {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<Document>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Document>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_document(document: *mut Document) {
    unsafe { crate::types::boxed::drop(document) };
}
