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

use std::ptr;

use num_traits::FromPrimitive;
use xayn_discovery_engine_core::document::UserReaction;

/// Create a rust `Option<UserReaction>`  initialized to `None`.
///
/// Return `0` if the discriminant it was out-of-range `1` otherwise.
///
/// # Safety
///
/// - It must be valid to write a `Option<UserReaction>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_option_user_reaction_some_at(
    place: *mut Option<UserReaction>,
    user_reaction: u8,
) -> u8 {
    let opt_reaction = UserReaction::from_u8(user_reaction);
    unsafe { place.write(opt_reaction) }
    u8::from(opt_reaction.is_some())
}

/// Create a rust `Option<UserReaction>`  initialized to `Some(reaction)`.
///
/// # Safety
///
/// - It must be valid to write a `Option<UserReaction>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
#[no_mangle]
pub unsafe extern "C" fn init_option_user_reaction_none_at(place: *mut Option<UserReaction>) {
    unsafe {
        place.write(None);
    }
}

/// Returns a ptr. to the `UserReaction` in the `Some`, or `nullptr` if the option is `None`.
///
/// # Safety
///
/// - Place must point to a sound instance of `Option<UserReaction>`
#[no_mangle]
pub unsafe extern "C" fn get_option_user_reaction_some_ptr(
    place: &mut Option<UserReaction>,
) -> *mut UserReaction {
    match place {
        Some(reaction) => reaction,
        None => ptr::null_mut(),
    }
}

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
