// Copyright 2021 Xayn AG
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

use std::ops::RangeInclusive;

use itertools::Itertools;
use ndarray::{Array2, ArrayView1};
pub use xayn_ai_bert::{Embedding1 as Embedding, MalformedBytesEmbedding};

/// See [`pairwise_cosine_similarity`] for details.
pub(crate) const MAXIMUM_COSINE_SIMILARITY: f32 = 1.0;

/// See [`pairwise_cosine_similarity`] for details.
pub(crate) const MINIMUM_COSINE_SIMILARITY: f32 = -1.0;

/// See [`pairwise_cosine_similarity`] for details.
#[cfg_attr(not(doc), allow(unreachable_pub))]
pub const COSINE_SIMILARITY_RANGE: RangeInclusive<f32> =
    MINIMUM_COSINE_SIMILARITY..=MAXIMUM_COSINE_SIMILARITY;

/// Computes the pairwise cosine similarities of vectors.
///
/// Each value is bounded in `[-1, 1]`. The zero vector is always "similar" to all other vectors,
/// thus will yield a similarity of 1.
///
/// # Panics
/// Panics if the vectors don't consist solely of real values or their shapes don't match.
///
/// Only use this with [`TrustedLen`] iterators, otherwise indexing may panic. The bound can't be
/// named currently, because the trait is nightly-gated. [`ExactSizeIterator`] isn't a feasible and
/// trusted replacement, for example it isn't implemented for [`Chain`]ed iterators.
///
/// [`TrustedLen`]: std::iter::TrustedLen
/// [`ExactSizeIterator`]: std::iter::ExactSizeIterator
/// [`Chain`]: std::iter::Chain
pub fn pairwise_cosine_similarity<'a, I>(iter: I) -> Array2<f32>
where
    I: IntoIterator<Item = ArrayView1<'a, f32>>,
    I::IntoIter: Clone,
{
    let iter = iter.into_iter();
    let size = match iter.size_hint() {
        (lower, Some(upper)) if lower == upper => lower,
        (lower, None) if lower == usize::MAX => lower,
        _ => unimplemented!("I::IntoIter: TrustedLen"),
    };

    let mut similarities = Array2::ones((size, size));
    for ((i, a), (j, b)) in iter.clone().enumerate().cartesian_product(iter.enumerate()) {
        similarities[[i, j]] = a.dot(&b).clamp(-1., 1.);
        similarities[[j, i]] = similarities[[i, j]];
    }

    similarities
}

/// Computes the dot product of two vectors.
pub fn normalized_dot_product(a: ArrayView1<'_, f32>, b: ArrayView1<'_, f32>) -> f32 {
    if a.iter().any(|&v| v != 0.) || b.iter().any(|&v| v != 0.) {
        a.dot(&b).clamp(-1., 1.)
    } else {
        1.
    }
}

#[cfg(test)]
mod tests {
    use ndarray::arr1;
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;
    use crate::utils::normalize_array;

    #[test]
    fn test_cosine_similarity_zero() {
        let embedding_a = Embedding::from(arr1(&normalize_array([1., 2., 3.])));
        let embedding_b = Embedding::from(arr1(&normalize_array([0., 0., 0.])));
        assert_approx_eq!(
            f32,
            normalized_dot_product(embedding_a.view(), embedding_b.view()),
            1.0
        );
    }
}
