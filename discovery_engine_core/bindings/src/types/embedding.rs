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

//! FFI functions for handling embeddings.

use std::process::abort;

use ndarray::Array;

use crate::types::slice::boxed_slice_from_raw_parts;
use xayn_discovery_engine_core::document::Embedding;

use super::primitives::FfiUsize;

/// Creates a rust `Embedding` with given capacity at given memory address.
///
/// # Safety
///
/// - It must be valid to write an `Embedding` instance to given pointer.
/// - The passed in slice must represent a `Box<[f32]>` and transfers ownership,
///   it must be fully initialized.
#[no_mangle]
pub unsafe extern "C" fn init_embedding_at(
    place: *mut Embedding,
    owning_ptr: *mut f32,
    len: FfiUsize,
) {
    let boxed_slice = unsafe { boxed_slice_from_raw_parts::<f32>(owning_ptr, len) };
    let embedding = Embedding::from(Array::from(boxed_slice));
    unsafe {
        place.write(embedding);
    }
}

/// Initializes a rust `Option<Embedding>` to `Some(embedding)` based on given slice at given place.
///
/// # Safety
///
/// - It must be valid to write an `Embedding` instance to given pointer.
/// - The passed in slice must represent a `Box<[f32]>` and transfers ownership,
///   and it must be fully initialized.
#[no_mangle]
pub unsafe extern "C" fn init_option_embedding_some_at(
    place: *mut Option<Embedding>,
    owning_ptr: *mut f32,
    len: FfiUsize,
) {
    let boxed_slice = unsafe { boxed_slice_from_raw_parts::<f32>(owning_ptr, len) };
    let value = Some(Embedding::from(Array::from(boxed_slice)));
    unsafe {
        place.write(value);
    }
}

/// Initializes a rust `Option<Embedding>` to `None`.
///
/// # Safety
///
/// - It must be valid to write an `Embedding` instance to given pointer.
#[no_mangle]
pub unsafe extern "C" fn init_option_embedding_none_at(place: *mut Option<Embedding>) {
    unsafe {
        place.write(None);
    }
}

/// Returns a pointer to the begin of the `[f32]` backing the `Embedding`
///
/// # Safety
///
/// The pointer must point to a valid [`Embedding`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_embedding_buffer(embedding: *const Embedding) -> *const f32 {
    unsafe { &*embedding }.as_ptr()
}

/// Returns len of the given embedding.
///
/// # Safety
///
/// The pointer must point to a valid [`Embedding`] instance.
///
/// # Aborts
///
/// Aborts if the embedding is not "contiguous and in standard order".
//Note: Whether this is or isn't the case should (for our use case) be always
// the same independent of input data. Hence this should be caught by
// tests. If that isn't the case anymore it should be considered to
// change this interface, e.g. to support reading a buffer with strides.
#[no_mangle]
pub unsafe extern "C" fn get_embedding_buffer_len(embedding: *mut Embedding) -> FfiUsize {
    let embedding = unsafe { &*embedding }.as_slice();
    #[allow(clippy::redundant_closure)] // there is a type mismatch if we remove the closure
    let len = embedding.unwrap_or_else(|| abort()).len();

    FfiUsize::from_usize_lossy(len)
}

/// Alloc an uninitialized `Box<Embedding>`.
#[no_mangle]
pub extern "C" fn alloc_uninitialized_embedding() -> *mut Embedding {
    super::boxed::alloc_uninitialized()
}

/// Drops a `Box<Embedding>`.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Embedding>` instance.
#[no_mangle]
pub unsafe extern "C" fn drop_embedding(embedding: *mut Embedding) {
    unsafe { super::boxed::drop(embedding) };
}

#[cfg(test)]
mod tests {
    use std::{ptr, slice};

    use ndarray::arr1;

    use crate::types::primitives::alloc_uninitialized_f32_slice;

    use super::*;

    #[test]
    fn test_reading_embedding_works() {
        let embedding = &mut arbitrary_embedding();
        let read = unsafe {
            let buffer_view = slice::from_raw_parts(
                get_embedding_buffer(embedding),
                get_embedding_buffer_len(embedding).to_usize(),
            );
            Embedding::from(Array::from_vec(buffer_view.to_owned()))
        };
        assert_eq!(*read, *embedding);
    }

    #[test]
    fn test_writing_embedding_works() {
        let embedding = arbitrary_embedding();
        let len = FfiUsize::from_usize_lossy(embedding.len());
        let place: &mut Embedding = &mut Embedding::from(arr1(&[]));
        unsafe {
            let data_ptr = alloc_uninitialized_f32_slice(len);
            ptr::copy(embedding.as_ptr(), data_ptr, len.to_usize());
            init_embedding_at(place, data_ptr, len);
        }
        assert_eq!(*place, embedding);
    }

    fn arbitrary_embedding() -> Embedding {
        Embedding::from(arr1(&[0.0, 1.2, 3.1, 0.4]))
    }
}
