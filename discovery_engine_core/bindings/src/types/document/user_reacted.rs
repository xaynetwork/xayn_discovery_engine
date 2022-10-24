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

use xayn_discovery_engine_core::document::{UserReacted, UserReaction};

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

/// Alloc an uninitialized `Box<UserReacted>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_user_reacted() -> *mut UserReacted {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<UserReacted>`.
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

    use xayn_discovery_engine_core::document::Id;

    use super::*;

    #[test]
    fn test_ids_have_same_layout() {
        let uuid_layout = Layout::new::<Uuid>();
        let document_id_layout = Layout::new::<Id>();
        assert_eq!(document_id_layout, uuid_layout);
    }
}
