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

//! FFI functions for handling [`InitConfig`] instances.

use std::ptr::addr_of_mut;

use xayn_discovery_engine_core::{storage::DartMigrationData, InitConfig};
use xayn_discovery_engine_providers::Market;

use super::primitives::FfiUsize;

/// Returns a pointer to the `api_key` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_api_key(place: *mut InitConfig) -> *mut String {
    unsafe { addr_of_mut!((*place).api_key) }
}

/// Returns a pointer to the `api_base_url` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_api_base_url(place: *mut InitConfig) -> *mut String {
    unsafe { addr_of_mut!((*place).api_base_url) }
}

/// Returns a pointer to the `news_provider_path` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_news_provider_path(
    place: *mut InitConfig,
) -> *mut String {
    unsafe { addr_of_mut!((*place).news_provider_path) }
}

/// Returns a pointer to the `headlines_provider_path` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_headlines_provider_path(
    place: *mut InitConfig,
) -> *mut String {
    unsafe { addr_of_mut!((*place).headlines_provider_path) }
}

/// Returns a pointer to the `markets` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_markets(place: *mut InitConfig) -> *mut Vec<Market> {
    unsafe { addr_of_mut!((*place).markets) }
}

/// Returns a pointer to the `bert` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_bert(place: *mut InitConfig) -> *mut String {
    unsafe { addr_of_mut!((*place).bert) }
}

/// Returns a pointer to the `max_docs_per_feed_batch` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_max_docs_per_feed_batch(
    place: *mut InitConfig,
) -> *mut FfiUsize {
    unsafe { addr_of_mut!((*place).max_docs_per_feed_batch) }.cast()
}

/// Returns a pointer to the `max_docs_per_search_batch` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_max_docs_per_search_batch(
    place: *mut InitConfig,
) -> *mut FfiUsize {
    unsafe { addr_of_mut!((*place).max_docs_per_search_batch) }.cast()
}

/// Returns a pointer to the `de_config` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_de_config(
    place: *mut InitConfig,
) -> *mut Option<String> {
    unsafe { addr_of_mut!((*place).de_config) }
}

/// Returns a pointer to the `log_file` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_log_file(
    place: *mut InitConfig,
) -> *mut Option<String> {
    unsafe { addr_of_mut!((*place).log_file) }
}

/// Returns a pointer to the `data_dir` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_data_dir(place: *mut InitConfig) -> *mut String {
    unsafe { addr_of_mut!((*place).data_dir) }
}

/// Returns a pointer to the `use_ephemeral_db` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_use_ephemeral_db(place: *mut InitConfig) -> *mut u8 {
    unsafe { addr_of_mut!((*place).use_ephemeral_db).cast() }
}

/// Returns a pointer to the `dart_migration_data` field of a configuration.
///
/// # Safety
///
/// The pointer must point to a valid [`InitConfig`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn init_config_place_of_dart_migration_data(
    place: *mut InitConfig,
) -> *mut Option<DartMigrationData> {
    unsafe { addr_of_mut!((*place).dart_migration_data) }
}

/// Alloc an uninitialized `Box<InitConfig>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_init_config() -> *mut InitConfig {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<InitConfig>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<InitConfig>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_init_config(init_config: *mut InitConfig) {
    unsafe { crate::types::boxed::drop(init_config) };
}
