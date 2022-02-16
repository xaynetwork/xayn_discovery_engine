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

//! FFI functions for handling [`UserReaction`] fields.

use xayn_discovery_engine_core::document::UserReaction;

/// Alloc an uninitialized `Box<UserReaction>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_user_reaction() -> *mut UserReaction {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<UserReaction>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<UserReaction>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_user_reaction(reaction: *mut UserReaction) {
    unsafe { crate::types::boxed::drop(reaction) };
}
