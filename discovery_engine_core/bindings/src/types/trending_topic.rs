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

//! FFI functions for handling trending topics.

use std::ptr::addr_of_mut;

use url::Url;
use uuid::Uuid;

use xayn_discovery_engine_core::document::{Embedding, TrendingTopic};

/// Returns a pointer to the `id` field of a trending topic.
///
/// # Safety
///
/// The pointer must point to a valid [`TrendingTopic`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn trending_topic_place_of_id(place: *mut TrendingTopic) -> *mut Uuid {
    unsafe { addr_of_mut!((*place).id) }.cast::<Uuid>()
}

/// Returns a pointer to the `smbert_embedding` field of a trending topic.
///
/// # Safety
///
/// The pointer must point to a valid [`TrendingTopic`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn trending_topic_place_of_smbert_embedding(
    place: *mut TrendingTopic,
) -> *mut Embedding {
    unsafe { addr_of_mut!((*place).smbert_embedding) }
}

/// Returns a pointer to the `name` field of a trending topic.
///
/// # Safety
///
/// The pointer must point to a valid [`TrendingTopic`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn trending_topic_place_of_name(place: *mut TrendingTopic) -> *mut String {
    unsafe { addr_of_mut!((*place).name) }
}

/// Returns a pointer to the `query` field of a trending topic.
///
/// # Safety
///
/// The pointer must point to a valid [`TrendingTopic`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn trending_topic_place_of_query(place: *mut TrendingTopic) -> *mut String {
    unsafe { addr_of_mut!((*place).query) }
}

/// Returns a pointer to the `image` field of a trending topic.
///
/// # Safety
///
/// The pointer must point to a valid [`TrendingTopic`] memory object,
/// which might be uninitialized.
#[no_mangle]
pub unsafe extern "C" fn trending_topic_place_of_image(
    place: *mut TrendingTopic,
) -> *mut Option<Url> {
    unsafe { addr_of_mut!((*place).image) }
}

/// Alloc an uninitialized `Box<TrendingTopic>`, mainly used for testing.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_trending_topic() -> *mut TrendingTopic {
    crate::types::boxed::alloc_uninitialized()
}

/// Drops a `Box<TrendingTopic>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<TrendingTopic>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_trending_topic(topic: *mut TrendingTopic) {
    unsafe { crate::types::boxed::drop(topic) };
}
