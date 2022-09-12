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

use std::ptr::addr_of_mut;

use xayn_discovery_engine_core::{
    document::WeightedSource,
    storage2::{DartMigrationData, MigrationDocument},
};

use super::migration_search::MigrationSearch;

/// Returns a pointer to the `engine_state` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_engine_state(
    place: *mut DartMigrationData,
) -> *mut Option<Vec<u8>> {
    unsafe { addr_of_mut!((*place).engine_state) }
}

/// Returns a pointer to the `trusted_sources` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_trusted_sources(
    place: *mut DartMigrationData,
) -> *mut Vec<String> {
    unsafe { addr_of_mut!((*place).trusted_sources) }
}

/// Returns a pointer to the `excluded_sources` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_excluded_sources(
    place: *mut DartMigrationData,
) -> *mut Vec<String> {
    unsafe { addr_of_mut!((*place).excluded_sources) }
}

/// Returns a pointer to the `search` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_search(
    place: *mut DartMigrationData,
) -> *mut Option<MigrationSearch> {
    unsafe { addr_of_mut!((*place).search) }
}

/// Returns a pointer to the `documents` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_documents(
    place: *mut DartMigrationData,
) -> *mut Vec<MigrationDocument> {
    unsafe { addr_of_mut!((*place).documents) }
}

/// Returns a pointer to the `reacted_sources` field of a [`DartMigrationData`].
///
/// # Safety
///
/// The pointer must point to a valid [`DartMigrationData`] memory object,
/// it might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn dart_migration_data_place_of_reacted_sources(
    place: *mut DartMigrationData,
) -> *mut Vec<WeightedSource> {
    unsafe { addr_of_mut!((*place).reacted_sources) }
}

/// Alloc an uninitialized `Box<DartMigrationData>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_dart_migration_data() -> *mut DartMigrationData {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<DartMigrationData>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<DartMigrationData>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_dart_migration_data(data: *mut DartMigrationData) {
    unsafe { crate::types::boxed::drop(data) };
}
