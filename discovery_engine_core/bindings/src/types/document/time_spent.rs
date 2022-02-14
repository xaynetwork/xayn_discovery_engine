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

//! FFI functions for handling `TimeSpent` structs.

use core::document::{Embedding, TimeSpent, UserReaction};
use std::{ptr::addr_of_mut, time::Duration};

use uuid::Uuid;

/// Returns a pointer to the `id` field of a [`TimeSpent`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`TimeSpent`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn time_spent_place_of_id(place: *mut TimeSpent) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast::<Uuid>()
}

/// Returns a pointer to the `smbert_embedding` field of a [`TimeSpent`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`TimeSpent`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn time_spent_place_of_smbert_embedding(
    place: *mut TimeSpent,
) -> *mut Embedding {
    unsafe { addr_of_mut!((*place).smbert_embedding) }
}

/// Returns a pointer to the `time` field of a [`TimeSpent`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`TimeSpent`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn time_spent_place_of_time(place: *mut TimeSpent) -> *mut Duration {
    unsafe { addr_of_mut!((*place).time) }
}

/// Returns a pointer to the `reaction` field of a [`TimeSpent`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`TimeSpent`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn time_spent_place_of_reaction(place: *mut TimeSpent) -> *mut UserReaction {
    unsafe { addr_of_mut!((*place).reaction) }
}

/// Alloc an uninitialized `Box<TimeSpent>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_time_spend() -> *mut TimeSpent {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<TimeSpent>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<TimeSpent>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_time_spent(time_spent: *mut TimeSpent) {
    unsafe { crate::types::boxed::drop(time_spent) };
}
