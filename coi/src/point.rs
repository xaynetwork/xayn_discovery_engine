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
use uuid::Uuid;
use xayn_ai_bert::{InvalidEmbedding, NormalizedEmbedding};

use crate::stats::Stats;

/// A unique identifier of a [`Coi`].
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type), sqlx(transparent))]
pub struct Id(Uuid);

impl Id {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// A center of interest.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coi {
    pub id: Id,
    pub point: NormalizedEmbedding,
    pub stats: Stats,
}

impl Coi {
    /// Creates a coi.
    pub fn new(id: Id, point: NormalizedEmbedding, time: DateTime<Utc>) -> Self {
        Self {
            id,
            point,
            stats: Stats::new(time),
        }
    }

    /// Shifts the coi point towards another point by a factor.
    pub fn shift_point(
        &mut self,
        towards: &NormalizedEmbedding,
        shift_factor: f32,
    ) -> Result<&mut Self, InvalidEmbedding> {
        self.point = (&self.point * (1. - shift_factor) + towards * shift_factor).normalize()?;
        Ok(self)
    }
}

/// Finds the most similar [`Coi`] for the given embedding.
///
/// The similarity ranges in the interval `[-1., 1.]`.
pub(super) fn find_closest_coi_index(
    cois: &[Coi],
    embedding: &NormalizedEmbedding,
) -> Option<(usize, f32)> {
    let mut similarities = cois
        .iter()
        .map(|coi| embedding.dot_product(&coi.point))
        .enumerate()
        .collect_vec();
    similarities.sort_by(|(_, s1), (_, s2)| s1.total_cmp(s2).reverse());

    similarities.first().copied()
}

/// Finds the most similar [`Coi`] for the given embedding.
pub(super) fn find_closest_coi_mut<'a>(
    cois: &'a mut [Coi],
    embedding: &NormalizedEmbedding,
) -> Option<(&'a mut Coi, f32)> {
    find_closest_coi_index(cois, embedding)
        .map(move |(index, similarity)| (&mut cois[index], similarity))
}

#[cfg(test)]
pub(crate) mod tests {
    use xayn_test_utils::{assert_approx_eq, uuid::mock_uuid};

    use super::*;

    pub(crate) fn create_cois<const M: usize, const N: usize>(
        points: [[f32; N]; M],
        time: DateTime<Utc>,
    ) -> Vec<Coi> {
        points
            .into_iter()
            .enumerate()
            .map(|(id, point)| Coi::new(Id(mock_uuid(id)), point.try_into().unwrap(), time))
            .collect()
    }

    #[test]
    fn test_shift_coi_point_towards_other() {
        let mut cois = create_cois([[1., 1., 1.]], Utc::now());
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
    fn test_shift_coi_point_towards_self() {
        let mut cois = create_cois([[1., 1., 1.]], Utc::now());
        let towards = cois[0].point.clone();
        let shift_factor = 0.1;
        cois[0].shift_point(&towards, shift_factor).unwrap();
        assert_approx_eq!(f32, cois[0].point, towards);
    }

    #[test]
    fn test_find_closest_coi_single() {
        let cois = create_cois([[1., 2., 3.]], Utc::now());
        let embedding = [1., 5., 9.].try_into().unwrap();
        let (index, similarity) = find_closest_coi_index(&cois, &embedding).unwrap();
        assert_eq!(index, 0);
        assert_approx_eq!(f32, similarity, 0.981_810_57);
    }

    #[test]
    fn test_find_closest_coi() {
        let cois = create_cois([[6., 1., 8.], [12., 4., 0.], [0., 4., 13.]], Utc::now());
        let embedding = [1., 5., 9.].try_into().unwrap();
        let (index, similarity) = find_closest_coi_index(&cois, &embedding).unwrap();
        assert_eq!(index, 2);
        assert_approx_eq!(f32, similarity, 0.973_739_56);
    }

    #[test]
    fn test_find_closest_coi_equal() {
        let cois = create_cois([[1., 2., 3.]], Utc::now());
        let embedding = [1., 2., 3.].try_into().unwrap();
        let (index, similarity) = find_closest_coi_index(&cois, &embedding).unwrap();
        assert_eq!(index, 0);
        assert_approx_eq!(f32, similarity, 1.);
    }

    #[test]
    fn test_find_closest_coi_index_empty() {
        let embedding = [1., 2., 3.].try_into().unwrap();
        assert!(find_closest_coi_index(&[], &embedding).is_none());
    }
}
