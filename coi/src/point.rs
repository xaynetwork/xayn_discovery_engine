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

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use xayn_ai_bert::{InvalidEmbedding, NormalizedEmbedding};

use crate::{id::CoiId, stats::CoiStats, utils::nan_safe_f32_cmp_desc};

/// A positive `CoI`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PositiveCoi {
    pub id: CoiId,
    pub point: NormalizedEmbedding,
    pub stats: CoiStats,
}

/// A negative `CoI`.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NegativeCoi {
    pub id: CoiId,
    pub point: NormalizedEmbedding,
    pub last_view: DateTime<Utc>,
}

/// Common `CoI` properties and functionality.
pub trait CoiPoint {
    /// Creates a coi.
    fn new(id: CoiId, point: NormalizedEmbedding) -> Self;

    /// Gets the coi id.
    fn id(&self) -> CoiId;

    /// Gets the coi point.
    fn point(&self) -> &NormalizedEmbedding;

    /// Shifts the coi point towards another point by a factor.
    fn shift_point(
        &mut self,
        towards: &NormalizedEmbedding,
        shift_factor: f32,
    ) -> Result<&mut Self, InvalidEmbedding>;
}

macro_rules! impl_coi_point {
    ($($(#[$attr:meta])* $coi:ty $({ $($field:ident: $value:expr),* $(,)? })?),* $(,)?) => {
        $(
            $(#[$attr])*
            impl CoiPoint for $coi {
                fn new(id: CoiId, point: NormalizedEmbedding) -> Self {
                    Self {
                        id,
                        point,
                        $($($field: $value),*)?,
                    }
                }

                fn id(&self) -> CoiId {
                    self.id
                }

                fn point(&self) -> &NormalizedEmbedding {
                    &self.point
                }

                fn shift_point(
                    &mut self,
                    towards: &NormalizedEmbedding,
                    shift_factor: f32,
                ) -> Result<&mut Self, InvalidEmbedding> {
                    self.point =
                        (&self.point * (1. - shift_factor) + towards * shift_factor).normalize()?;
                    Ok(self)
                }
            }
        )*
    };
}

impl_coi_point! {
    PositiveCoi { stats: CoiStats::new() },
    NegativeCoi { last_view: Utc::now() },
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi_index(
    cois: &[impl CoiPoint],
    embedding: &NormalizedEmbedding,
) -> Option<(usize, f32)> {
    let mut similarities = cois
        .iter()
        .map(|coi| embedding.dot_product(coi.point()))
        .enumerate()
        .collect_vec();
    similarities.sort_by(|(_, a), (_, b)| nan_safe_f32_cmp_desc(a, b));

    similarities.first().copied()
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi<'coi, CP>(
    cois: &'coi [CP],
    embedding: &NormalizedEmbedding,
) -> Option<(&'coi CP, f32)>
where
    CP: CoiPoint,
{
    find_closest_coi_index(cois, embedding).map(|(index, similarity)| (&cois[index], similarity))
}

/// Finds the most similar centre of interest (`CoI`) for the given embedding.
pub(super) fn find_closest_coi_mut<'coi, CP>(
    cois: &'coi mut [CP],
    embedding: &NormalizedEmbedding,
) -> Option<(&'coi mut CP, f32)>
where
    CP: CoiPoint,
{
    find_closest_coi_index(cois, embedding)
        .map(move |(index, similarity)| (&mut cois[index], similarity))
}

#[cfg(test)]
pub(crate) mod tests {
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;

    pub(crate) fn create_cois<const M: usize, const N: usize, CP>(points: [[f32; N]; M]) -> Vec<CP>
    where
        CP: CoiPoint,
    {
        points
            .into_iter()
            .enumerate()
            .map(|(id, point)| CP::new(CoiId::mocked(id), point.try_into().unwrap()))
            .collect()
    }

    pub(crate) fn create_pos_cois<const M: usize, const N: usize>(
        points: [[f32; N]; M],
    ) -> Vec<PositiveCoi> {
        create_cois(points)
    }

    pub(crate) fn create_neg_cois<const M: usize, const N: usize>(
        points: [[f32; N]; M],
    ) -> Vec<NegativeCoi> {
        create_cois(points)
    }

    #[test]
    fn test_shift_coi_point() {
        let mut cois = create_pos_cois([[1., 1., 1.]]);
        let towards = [2., 3., 4.].try_into().unwrap();
        let shift_factor = 0.1;
        cois[0].shift_point(&towards, shift_factor).unwrap();
        assert_approx_eq!(
            f32,
            cois[0].point,
            [0.558_521_4, 0.577_149_87, 0.595_778_35],
        );
    }

    #[test]
    fn test_find_closest_coi_single() {
        let cois = create_pos_cois([[1., 2., 3.]]);
        let embedding = [1., 5., 9.].try_into().unwrap();
        let (index, similarity) = find_closest_coi_index(&cois, &embedding).unwrap();
        assert_eq!(index, 0);
        assert_approx_eq!(f32, similarity, 0.981_810_57);
    }

    #[test]
    fn test_find_closest_coi() {
        let cois = create_pos_cois([[6., 1., 8.], [12., 4., 0.], [0., 4., 13.]]);
        let embedding = [1., 5., 9.].try_into().unwrap();
        let (closest, similarity) = find_closest_coi(&cois, &embedding).unwrap();
        assert_approx_eq!(f32, closest.point, cois[2].point);
        assert_approx_eq!(f32, similarity, 0.973_739_56);
    }

    #[test]
    fn test_find_closest_coi_equal() {
        let cois = create_pos_cois([[1., 2., 3.]]);
        let embedding = [1., 2., 3.].try_into().unwrap();
        let (closest, similarity) = find_closest_coi(&cois, &embedding).unwrap();
        assert_approx_eq!(f32, closest.point, cois[0].point);
        assert_approx_eq!(f32, similarity, 1.);
    }

    #[test]
    fn test_find_closest_coi_index_empty() {
        let embedding = [1., 2., 3.].try_into().unwrap();
        assert!(find_closest_coi_index(&[] as &[PositiveCoi], &embedding).is_none());
    }
}
