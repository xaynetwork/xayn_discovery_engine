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

//! Provides bindings to `crate::storage2::Search` renamed to `MigrationSearch`.
//!
//! The renaming is necessary to avoid ffi naming collisions. In the ffi this
//! type is only used for the dart->rust migration.

use std::ptr::addr_of_mut;

use xayn_discovery_engine_core::storage2::{Search, SearchBy};

//cbindgen:ignore
pub type MigrationSearch = Search;

/// Initializes an `Option<MigrationSearch>` to `None` at given place.
///
/// # Safety
///
/// The pointer must point to a valid [`Option<MigrationSearch>`] memory object, which
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_option_migration_search_none_at(place: *mut Option<MigrationSearch>) {
    unsafe {
        place.write(None);
    }
}

/// Initializes an `Option<MigrationSearch>` to `Some(search)` at given place.
///
/// The boxed search is moved into this function.
///
/// # Safety
///
/// The pointer must point to a valid [`Option<MigrationSearch>`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_option_migration_search_some_at(
    place: *mut Option<MigrationSearch>,
    search: Box<MigrationSearch>,
) {
    unsafe {
        place.write(Some(*search));
    }
}

/// Returns a pointer to the `search_by` field of a [`MigrationSearch`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationSearch`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_search_place_of_search_by(
    place: *mut MigrationSearch,
) -> *mut SearchBy {
    unsafe { addr_of_mut!((*place).search_by) }
}

/// Returns a pointer to the `search_term` field of a [`MigrationSearch`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationSearch`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_search_place_of_search_term(
    place: *mut MigrationSearch,
) -> *mut String {
    unsafe { addr_of_mut!((*place).search_term) }
}

/// Returns a pointer to the `size` field of a `Paging` memory object contained in a [`MigrationSearch`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationSearch`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_search_place_of_page_size(
    place: *mut MigrationSearch,
) -> *mut u32 {
    unsafe { addr_of_mut!((*place).paging.size) }
}

/// Returns a pointer to the `next_page` field of a `Paging` memory object contained in a [`MigrationSearch`] memory object.
///
/// # Safety
///
/// The pointer must point to a valid [`MigrationSearch`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn migration_search_place_of_next_page(
    place: *mut MigrationSearch,
) -> *mut u32 {
    unsafe { addr_of_mut!((*place).paging.next_page) }
}

/// Alloc an uninitialized `Box<MigrationSearch>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_migration_search() -> *mut MigrationSearch {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<MigrationSearch>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<MigrationSearch>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_migration_search(search: *mut MigrationSearch) {
    unsafe { crate::types::boxed::drop(search) };
}
