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

use std::time::SystemTime;

use derivative::Derivative;
use serde::{Deserialize, Serialize};

use crate::{
    embedding::{normalized_dot_product, Embedding},
    id::CoiId,
    stats::CoiStats,
    utils::system_time_now,
};

/// A positive `CoI`.
#[derive(Clone, Debug, Derivative, Deserialize, Serialize)]
#[derivative(PartialEq)]
pub struct PositiveCoi {
    pub id: CoiId,
    pub point: Embedding,
    #[derivative(PartialEq = "ignore")]
    pub stats: CoiStats,
}

impl PositiveCoi {
    /// Creates a positive `CoI`.
    pub fn new(id: impl Into<CoiId>, point: impl Into<Embedding>) -> Self {
        Self {
            id: id.into(),
            point: point.into(),
            stats: CoiStats::new(),
        }
    }
}

/// A negative `CoI`.
#[derive(Clone, Debug, Derivative, Deserialize, Serialize)]
#[derivative(PartialEq)]
pub struct NegativeCoi {
    pub id: CoiId,
    pub point: Embedding,
    #[derivative(PartialEq = "ignore")]
    pub last_view: SystemTime,
}

impl NegativeCoi {
    /// Creates a negative `CoI`.
    pub fn new(id: impl Into<CoiId>, point: impl Into<Embedding>) -> Self {
        Self {
            id: id.into(),
            point: point.into(),
            last_view: system_time_now(),
        }
    }
}

/// Common `CoI` properties and functionality.
pub trait CoiPoint {
    /// Gets the coi id.
    fn id(&self) -> CoiId;

    /// Gets the coi point.
    fn point(&self) -> &Embedding;

    /// Shifts the coi point towards another point by a factor.
    fn shift_point(&mut self, towards: &Embedding, shift_factor: f32) -> &mut Self;
}

macro_rules! impl_coi_point {
    ($($(#[$attr:meta])* $coi:ty),* $(,)?) => {
        $(
            $(#[$attr])*
            impl CoiPoint for $coi {
                fn id(&self) -> CoiId {
                    self.id
                }

                fn point(&self) -> &Embedding {
                    &self.point
                }

                fn shift_point(&mut self, towards: &Embedding, shift_factor: f32) -> &mut Self {
                    self.point *= 1. - shift_factor;
                    self.point += towards * shift_factor;
                    self
                }
            }
        )*
    };
}

impl_coi_point! {
    PositiveCoi,
    NegativeCoi,
}

/// The `CoI`s of a user.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UserInterests {
    pub positive: Vec<PositiveCoi>,
    pub negative: Vec<NegativeCoi>,
}

impl UserInterests {
    /// Checks if all user interests are empty.
    pub fn is_empty(&self) -> bool {
        self.positive.is_empty() && self.negative.is_empty()
    }
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi_index(
    cois: &[impl CoiPoint],
    embedding: &Embedding,
) -> Option<(usize, f32)> {
    if cois.is_empty() {
        return None;
    }

    let mut similarities = cois
        .iter()
        .map(|coi| normalized_dot_product(embedding.view(), coi.point().view()))
        .enumerate()
        .collect::<Vec<_>>();

    similarities.sort_by(|(_, this), (_, other)| this.partial_cmp(other).unwrap().reverse());
    Some(similarities[0])
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi<'coi, CP>(
    cois: &'coi [CP],
    embedding: &Embedding,
) -> Option<(&'coi CP, f32)>
where
    CP: CoiPoint,
{
    find_closest_coi_index(cois, embedding).map(|(index, similarity)| (&cois[index], similarity))
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi_mut<'coi, CP>(
    cois: &'coi mut [CP],
    embedding: &Embedding,
) -> Option<(&'coi mut CP, f32)>
where
    CP: CoiPoint,
{
    find_closest_coi_index(cois, embedding)
        .map(move |(index, similarity)| (&mut cois[index], similarity))
}

#[cfg(test)]
pub(crate) mod tests {
    use ndarray::{arr1, FixedInitializer};
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;
    use crate::utils::normalize_array;

    pub(crate) trait CoiPointConstructor {
        fn new(id: impl Into<CoiId>, point: impl Into<Embedding>) -> Self;
    }

    impl CoiPointConstructor for PositiveCoi {
        fn new(id: impl Into<CoiId>, point: impl Into<Embedding>) -> Self {
            Self::new(id, point)
        }
    }

    impl CoiPointConstructor for NegativeCoi {
        fn new(id: impl Into<CoiId>, point: impl Into<Embedding>) -> Self {
            Self::new(id, point)
        }
    }

    fn create_cois<FI: FixedInitializer<Elem = f32>, CP: CoiPointConstructor>(
        points: &[FI],
    ) -> Vec<CP> {
        if FI::len() == 0 {
            return Vec::new();
        }

        points
            .iter()
            .enumerate()
            .map(|(id, point)| CP::new(CoiId::mocked(id), arr1(point.as_init_slice())))
            .collect()
    }

    pub(crate) fn create_pos_cois(
        points: &[impl FixedInitializer<Elem = f32>],
    ) -> Vec<PositiveCoi> {
        create_cois(points)
    }

    pub(crate) fn create_neg_cois(
        points: &[impl FixedInitializer<Elem = f32>],
    ) -> Vec<NegativeCoi> {
        create_cois(points)
    }

    #[test]
    fn test_shift_coi_point() {
        let mut cois = create_pos_cois(&[[1., 1., 1.]]);
        let towards = arr1(&[2., 3., 4.]).into();
        let shift_factor = 0.1;

        cois[0].shift_point(&towards, shift_factor);
        assert_eq!(cois[0].point, arr1(&[1.1, 1.2, 1.3]));
    }

    // The test cases below were modeled after the scipy implementation of cosine similarity, e.g.
    //
    // from scipy.spatial import distance
    // # similarity is 1 - distance
    // print(1 - distance.cosine([1, 2, 3], [1, 5, 9])) # => 0.9818105397247233
    // (via https://docs.scipy.org/doc/scipy/reference/generated/scipy.spatial.distance.cosine.html)

    #[test]
    fn test_find_closest_coi_single() {
        let cois = create_pos_cois(&[normalize_array([1., 2., 3.])]);
        let (index, similarity) =
            find_closest_coi_index(&cois, &arr1(&normalize_array([1., 5., 9.])).into()).unwrap();

        assert_eq!(index, 0);
        assert_approx_eq!(f32, similarity, 0.981_810_57);
    }

    #[test]
    fn test_find_closest_coi() {
        let cois = create_pos_cois(&[
            normalize_array([6., 1., 8.]),
            normalize_array([12., 4., 0.]),
            normalize_array([0., 4., 13.]),
        ]);
        let (closest, similarity) =
            find_closest_coi(&cois, &arr1(&normalize_array([1., 5., 9.])).into()).unwrap();

        assert_eq!(closest.point, arr1(&normalize_array([0., 4., 13.])));
        assert_approx_eq!(f32, similarity, 0.973_739_56);
    }

    #[test]
    fn test_find_closest_coi_equal() {
        let cois = create_pos_cois(&[normalize_array([1., 2., 3.])]);
        let (closest, similarity) =
            find_closest_coi(&cois, &arr1(&normalize_array([1., 2., 3.])).into()).unwrap();

        assert_eq!(closest.point, arr1(&normalize_array([1., 2., 3.])));
        assert_approx_eq!(f32, similarity, 1.);
    }

    #[test]
    fn test_find_closest_coi_all_nan() {
        let cois = create_pos_cois(&[normalize_array([1., 2., 3.])]);
        let embedding = arr1(&[f32::NAN, f32::NAN, f32::NAN]).into();
        find_closest_coi_index(&cois, &embedding);
    }

    #[test]
    fn test_find_closest_coi_single_nan() {
        let embedding: Embedding = arr1(&[1., f32::NAN, 2.]).into();
        assert!(&embedding.normalize().is_err());
    }

    #[test]
    fn test_find_closest_coi_index_empty() {
        let embedding = arr1(&normalize_array([1., 2., 3.])).into();
        let coi = find_closest_coi_index(&[] as &[PositiveCoi], &embedding);
        assert!(coi.is_none());
    }
}
