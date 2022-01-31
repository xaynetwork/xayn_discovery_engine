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

use core::document::{Embedding, Embedding1};

use ndarray::Array;

use crate::types::slice::boxed_slice;

/// Creates a rust `Embedding1` with given capacity at given memory address.
///
/// The `Embedding1` is created at given address, not it's content.
///
/// Returns a pointer to the begin of the buffer of the embedding.
///
/// # Safety
///
/// - It must be valid to write an `Embedding1` instance to given pointer.
/// - The passed in slice must represent a `Box<[f32]>` and transfers ownership,
///   it must be fully initialized.
#[no_mangle]
pub unsafe extern "C" fn init_embedding1_at(
    place: *mut Embedding1,
    float_slice: *mut f32,
    len: usize,
) {
    let vec = unsafe { boxed_slice::<f32>(float_slice, len).into() };
    let embedding = Embedding(Array::from_vec(vec));
    unsafe {
        place.write(embedding);
    }
}

/// Returns a pointer to the begin of the `[f32]` backing the `Embedding1`
///
/// # Safety
///
/// The pointer must point to a valid [`Embedding1`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_embedding1_buffer(place: *mut Embedding1) -> *mut f32 {
    let embedding = unsafe { &mut *place };
    embedding.0.as_mut_ptr()
}

/// Returns len of given embeddings buffer.
///
/// # Safety
///
/// The pointer must point to a valid [`Embedding1`] instance.
#[no_mangle]
pub unsafe extern "C" fn get_embedding1_buffer_len(place: *mut Embedding1) -> usize {
    //WARNING: This holds for `Embedding1` but not for all possible `ndarray::Array<...>`.
    unsafe { &*place }.len()
}

/// Alloc an uninitialized `Box<String>`, mainly used for testing.
#[cfg(feature = "additional-ffi-methods")]
#[no_mangle]
pub extern "C" fn alloc_uninitialized_embedding1_box() -> *mut Embedding1 {
    use super::boxed::alloc_uninitialized_box;

    alloc_uninitialized_box()
}

/// Drops a `Box<String>`, mainly used for testing.
///
/// # Safety
///
/// The pointer must represent a valid `Box<Embedding1>` instance.
#[cfg(feature = "additional-ffi-methods")]
#[no_mangle]
pub unsafe extern "C" fn drop_embedding1_box(boxed: *mut Embedding1) {
    use super::boxed::drop_box;

    unsafe { drop_box(boxed) };
}

#[cfg(test)]
mod tests {
    use std::{ptr, slice};

    use ndarray::arr1;
    use rand::{distributions::Standard, prelude::Distribution, thread_rng, Rng};

    use crate::types::slice::alloc_uninitialized_f32_slice;

    use super::*;

    #[test]
    fn test_reading_embedding1_works() {
        let place = &mut random_embedding();
        let read = unsafe {
            let buffer_view = slice::from_raw_parts(
                get_embedding1_buffer(place),
                get_embedding1_buffer_len(place),
            );
            Embedding(Array::from_vec(buffer_view.to_owned()))
        };
        assert_eq!(place.0, read.0);
    }

    #[test]
    fn test_writing_embedding1_works() {
        let embedding = random_embedding();
        let len = embedding.len();
        let place: &mut Embedding1 = &mut Embedding(arr1(&[]));
        unsafe {
            let data_ptr = alloc_uninitialized_f32_slice(len);
            ptr::copy(embedding.as_ptr(), data_ptr, len);
            init_embedding1_at(place, data_ptr, len);
        }
        assert_eq!(embedding.0, place.0);
    }

    fn random_embedding() -> Embedding1 {
        let rng = &mut thread_rng();
        let len = rng.gen_range(1..1024);
        Embedding(Array::from_vec(
            Standard.sample_iter(rng).take(len).collect(),
        ))
    }
}
