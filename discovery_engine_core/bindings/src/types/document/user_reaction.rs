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

use core::document::UserReaction;

type IntRepr = u8;

/// Initializes an [`UserReaction`] field at given place.
///
/// # Safety
///
/// It must be valid to write an [`UserReaction`] to given pointer.
#[no_mangle]
pub unsafe extern "C" fn init_user_reaction_at(place: *mut UserReaction, reaction: IntRepr) -> u8 {
    let mut ok = true;
    let reaction = match reaction {
        0 => UserReaction::Neutral,
        1 => UserReaction::Positive,
        2 => UserReaction::Negative,
        _ => {
            ok = false;
            UserReaction::default()
        }
    };
    unsafe {
        place.write(reaction);
    }
    ok as u8
}

/// Gets the int representation of an [`UserReaction`].
///
/// # Safety
///
/// The pointer must point to a valid [`UserReaction`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_user_reaction(reaction: *mut UserReaction) -> IntRepr {
    let reaction = unsafe { &*reaction };
    match reaction {
        UserReaction::Neutral => 0,
        UserReaction::Positive => 1,
        UserReaction::Negative => 2,
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

#[cfg(test)]
mod tests {
    use std::alloc::Layout;

    use super::*;

    #[test]
    fn test_layout_is_correct() {
        let enum_layout = Layout::new::<UserReaction>();
        let u8_layout = Layout::new::<IntRepr>();
        assert_eq!(enum_layout, u8_layout);
    }

    #[test]
    fn test_reading_works() {
        let place = &mut UserReaction::Positive;
        let read = unsafe { get_user_reaction(place) };
        assert_eq!(*place as IntRepr, read);
    }

    #[test]
    fn test_writing_works() {
        let reaction = UserReaction::Positive;
        let place = &mut UserReaction::Negative;
        unsafe {
            assert_eq!(init_user_reaction_at(place, reaction as IntRepr), 1);
        }
        assert_eq!(*place, reaction);
    }

    #[test]
    fn test_writing_bounds_checks_work() {
        let place = &mut UserReaction::Negative;
        unsafe {
            assert_eq!(init_user_reaction_at(place, 100), 0);
        }
        assert_eq!(*place, UserReaction::default());
    }
}
