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

//! FFI functions for handling [`UserReacted`] instances.

use std::ptr::addr_of_mut;

use uuid::Uuid;

use xayn_discovery_engine_core::document::{Embedding, UserReacted, UserReaction};
use xayn_discovery_engine_providers::Market;

/// Returns a pointer to the `id` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_id(place: *mut UserReacted) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast::<Uuid>()
}

/// Returns a pointer to the `stack_id` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_stack_id(place: *mut UserReacted) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).stack_id) }.cast::<Uuid>()
}

/// Returns a pointer to the `snippet` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_snippet(place: *mut UserReacted) -> *mut String {
    unsafe { addr_of_mut!((*place).snippet) }
}

/// Returns a pointer to the `smbert_embedding` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_smbert_embedding(
    place: *mut UserReacted,
) -> *mut Embedding {
    unsafe { addr_of_mut!((*place).smbert_embedding) }
}

/// Returns a pointer to the `reaction` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_reaction(
    place: *mut UserReacted,
) -> *mut UserReaction {
    unsafe { addr_of_mut!((*place).reaction) }
}

/// Returns a pointer to the `market` field of an [`UserReacted`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReacted`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn user_reacted_place_of_market(place: *mut UserReacted) -> *mut Market {
    unsafe { addr_of_mut!((*place).market) }
}

/// Alloc an uninitialized `Box<UserReacted>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_user_reacted() -> *mut UserReacted {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<UserReacted>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<UserReacted>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_user_reacted(reacted: *mut UserReacted) {
    unsafe { crate::types::boxed::drop(reacted) };
}

#[cfg(test)]
mod tests {
    use std::alloc::Layout;

    use uuid::Uuid;

    use xayn_discovery_engine_core::{document, stack};

    #[test]
    fn test_ids_have_same_layout() {
        let uuid_layout = Layout::new::<Uuid>();
        let document_id_layout = Layout::new::<document::Id>();
        assert_eq!(document_id_layout, uuid_layout);
        let stack_id_layout = Layout::new::<stack::Id>();
        assert_eq!(stack_id_layout, uuid_layout);
    }
}
