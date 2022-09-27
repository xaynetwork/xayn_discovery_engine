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

use std::{ptr::addr_of_mut, time::Duration};

use uuid::Uuid;
use xayn_discovery_engine_ai::Embedding;
use xayn_discovery_engine_core::{
    document::{NewsResource, UserReaction},
    storage2::MigrationDocument,
};

use super::{
    date_time::DateTimeUtc,
    primitives::FfiUsize,
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
};

/// Returns a pointer to the `id` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_id(
    place: *mut MigrationDocument,
) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast()
}

/// Returns a pointer to the `stack_id` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_stack_id(
    place: *mut MigrationDocument,
) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).stack_id) }.cast()
}

/// Returns a pointer to the `smbert_embedding` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_smbert_embedding(
    place: *mut MigrationDocument,
) -> *mut Option<Embedding> {
    unsafe { addr_of_mut!((*place).smbert_embedding) }
}

/// Returns a pointer to the `reaction` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_reaction(
    place: *mut MigrationDocument,
) -> *mut UserReaction {
    unsafe { addr_of_mut!((*place).reaction) }
}

/// Returns a pointer to the `resource` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_resource(
    place: *mut MigrationDocument,
) -> *mut NewsResource {
    unsafe { addr_of_mut!((*place).resource) }
}

/// Inits the `is_active` field of an [`MigrationDocument`] memory object.
///
/// If `is_active` is `0` it's initialized to `false` else `true`.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_migration_document_is_active_at(
    place: *mut MigrationDocument,
    is_active: u8,
) {
    unsafe { addr_of_mut!((*place).is_active).write(is_active != 0) }
}

/// Inits the `is_searched` field of an [`MigrationDocument`] memory object.
///
/// If `is_searched` is `0` it's initialized to `false` else `true`.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_migration_document_is_searched_at(
    place: *mut MigrationDocument,
    is_searched: u8,
) {
    unsafe { addr_of_mut!((*place).is_searched).write(is_searched != 0) }
}

/// Returns a pointer to the `batch_index` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_batch_index(
    place: *mut MigrationDocument,
) -> *mut u32 {
    unsafe { addr_of_mut!((*place).batch_index) }
}

/// Returns a pointer to the `timestamp` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_timestamp(
    place: *mut MigrationDocument,
) -> *mut DateTimeUtc {
    unsafe { addr_of_mut!((*place).timestamp) }
}

/// Returns a pointer to the `web_view_time` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_web_view_time(
    place: *mut MigrationDocument,
) -> *mut Option<Duration> {
    unsafe { addr_of_mut!((*place).web_view_time) }
}

/// Returns a pointer to the `reader_view_time` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_reader_view_time(
    place: *mut MigrationDocument,
) -> *mut Option<Duration> {
    unsafe { addr_of_mut!((*place).reader_view_time) }
}

/// Returns a pointer to the `story_view_time` field of a [`MigrationDocument`].
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationDocument`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_document_place_of_story_view_time(
    place: *mut MigrationDocument,
) -> *mut Option<Duration> {
    unsafe { addr_of_mut!((*place).story_view_time) }
}

/// Initializes a `Vec<MigrationDocument>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[MigrationDocument]>`.
///
/// # Safety
///
/// - It must be valid to write a `Vec<MigrationDocument>` instance to given pointer,
///   and the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[MigrationDocument]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_migration_document_vec_at(
    place: *mut Vec<MigrationDocument>,
    slice_ptr: *mut MigrationDocument,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[MigrationDocument]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_migration_document_slice(
    len: FfiUsize,
) -> *mut MigrationDocument {
    alloc_uninitialized_slice(len)
}

/// Given a pointer to a [`MigrationDocument`] in a slice return the pointer to the next [`MigrationDocument`].
///
/// This also works for uninitialized migration document slices.
///
/// # Safety
///
/// The pointer must point to a valid `MigrationDocument` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_migration_document(
    place: *mut MigrationDocument,
) -> *mut MigrationDocument {
    unsafe { next_element(place) }
}
