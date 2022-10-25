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

//! FFI function for handling searches.

use std::ptr::addr_of_mut;

use xayn_discovery_engine_core::storage::models::SearchBy;

pub struct Search {
    pub by: SearchBy,
    pub term: String,
}

impl From<xayn_discovery_engine_core::SearchBy<'_>> for Search {
    fn from(search: xayn_discovery_engine_core::SearchBy<'_>) -> Self {
        match search {
            xayn_discovery_engine_core::SearchBy::Query(query) => Search {
                by: SearchBy::Query,
                term: query.keywords().join(" "),
            },
            xayn_discovery_engine_core::SearchBy::Topic(topic) => Search {
                by: SearchBy::Topic,
                term: topic.into_owned(),
            },
        }
    }
}

/// Alloc an uninitialized `Box<SearchBy>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_search_by() -> *mut SearchBy {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<SearchBy>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<SearchBy>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_search_by(search: *mut SearchBy) {
    unsafe { crate::types::boxed::drop(search) };
}

/// Returns a pointer to the `by` field of a [`Search`].
///
/// # Safety
///
/// The pointer must point to a valid [`Search`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn search_place_of_by(place: *mut Search) -> *mut SearchBy {
    unsafe { addr_of_mut!((*place).by) }
}

/// Returns a pointer to the `term` field of a [`Search`].
///
/// # Safety
///
/// The pointer must point to a valid [`Search`] memory object, it
/// might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn search_place_of_term(place: *mut Search) -> *mut String {
    unsafe { addr_of_mut!((*place).term) }
}

/// Alloc an uninitialized `Box<Search>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_search() -> *mut Search {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<Search>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Search>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_search(search: *mut Search) {
    unsafe { crate::types::boxed::drop(search) };
}
