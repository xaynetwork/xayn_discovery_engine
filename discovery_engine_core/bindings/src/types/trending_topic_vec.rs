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

//! FFI functions for handling slices and vectors of trending topics.

use xayn_discovery_engine_core::document::TrendingTopic;

use crate::types::{
    primitives::FfiUsize,
    slice::{alloc_uninitialized_slice, boxed_slice_from_raw_parts, next_element},
    vec::{get_vec_buffer, get_vec_len},
};

/// Initializes a `Vec<TrendingTopic>` at given place.
///
/// This moves the passed in slice into the vector,
/// i.e. `slice_ptr, slice_len` map to `Box<[TrendingTopic]>`.
///
/// # Safety
///
/// - It must be valid to write a `Vec<TrendingTopic>` instance to given pointer,
///   the pointer is expected to point to uninitialized memory.
/// - It must be valid to construct a `Box<[TrendingTopic]>` from given `slice_ptr`
///   and `slice_len`.
#[no_mangle]
pub unsafe extern "C" fn init_trending_topic_vec_at(
    place: *mut Vec<TrendingTopic>,
    slice_ptr: *mut TrendingTopic,
    slice_len: FfiUsize,
) {
    unsafe {
        place.write(Vec::from(boxed_slice_from_raw_parts(slice_ptr, slice_len)));
    }
}

/// Alloc an uninitialized `Box<[TrendingTopic]>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_trending_topic_slice(len: FfiUsize) -> *mut TrendingTopic {
    alloc_uninitialized_slice(len)
}

/// Given a pointer to a [`TrendingTopic`] in a slice return the pointer to the next [`TrendingTopic`].
///
/// This also works for uninitialized trending topic slices.
///
/// # Safety
///
/// The pointer must point to a valid `TrendingTopic` memory object, it might
/// be uninitialized. If it's the last object in an array the returned pointer
/// must not be dereferenced.
#[no_mangle]
pub unsafe extern "C" fn next_trending_topic(place: *mut TrendingTopic) -> *mut TrendingTopic {
    unsafe { next_element(place) }
}

/// Drop a `Box<[TrendingTopic]>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<[TrendingTopic]>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_trending_topic_slice(topics: *mut TrendingTopic, len: FfiUsize) {
    drop(unsafe { boxed_slice_from_raw_parts(topics, len) });
}

/// Returns the length of a `Box<Vec<TrendingTopic>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<TrendingTopic>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_trending_topic_vec_len(topics: *mut Vec<TrendingTopic>) -> FfiUsize {
    unsafe { get_vec_len(topics) }
}

/// Returns the `*mut TrendingTopic` to the beginning of the buffer of a `Box<Vec<TrendingTopic>>`.
///
/// # Safety
///
/// The pointer must point to a valid `Vec<TrendingTopic>` instance.
#[no_mangle]
pub unsafe extern "C" fn get_trending_topic_vec_buffer(
    topics: *mut Vec<TrendingTopic>,
) -> *mut TrendingTopic {
    unsafe { get_vec_buffer(topics) }
}

/// Drop a `Box<Vec<TrendingTopic>>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Vec<TrendingTopic>>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_trending_topic_vec(topics: *mut Vec<TrendingTopic>) {
    unsafe {
        crate::types::boxed::drop(topics);
    }
}
