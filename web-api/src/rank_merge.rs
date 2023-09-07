// Copyright 2023 Xayn AG
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

use std::{hash::Hash, ops::AddAssign};

use itertools::Itertools;
use xayn_web_api_shared::elastic::ScoreMap;

pub(crate) fn normalize_scores<K>(mut scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default();

    if max_score != 0. {
        for score in scores.values_mut() {
            *score /= max_score;
        }
    }

    scores
}

pub(crate) fn normalize_scores_if_max_gt_1<K>(mut scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let max_score = scores
        .values()
        .max_by(|l, r| l.total_cmp(r))
        .copied()
        .unwrap_or_default()
        .max(1.0);

    for score in scores.values_mut() {
        *score /= max_score;
    }

    scores
}

pub(crate) fn merge_scores_average_duplicates_only<K>(
    mut scores_1: ScoreMap<K>,
    scores_2: ScoreMap<K>,
) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    for (key, value) in scores_2 {
        scores_1
            .entry(key)
            .and_modify(|score| *score = (*score + value) / 2.)
            .or_insert(value);
    }
    scores_1
}

pub(crate) fn merge_scores_weighted<K>(
    scores: impl IntoIterator<Item = (f32, ScoreMap<K>)>,
) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    let weighted = scores.into_iter().flat_map(|(weight, mut scores)| {
        for score in scores.values_mut() {
            *score *= weight;
        }
        scores
    });
    collect_summing_repeated(weighted)
}

pub const DEFAULT_RRF_K: f32 = 60.;

/// Reciprocal Rank Fusion
pub fn rrf<K>(k: f32, scores: impl IntoIterator<Item = (f32, ScoreMap<K>)>) -> ScoreMap<K>
where
    K: Eq + Hash + Ord,
{
    let rrf_scores = scores.into_iter().flat_map(|(weight, scores)| {
        scores
            .into_iter()
            // For testing we want to make sure that in case of s1 == s2 we still get a
            // deterministic result, for this we use the key ordering for equal scores
            .sorted_by(|(k1, s1), (k2, s2)| s1.total_cmp(s2).then_with(|| k1.cmp(k2)).reverse())
            .enumerate()
            .map(move |(rank0, (document, _))| (document, rrf_score(k, rank0, weight)))
    });
    collect_summing_repeated(rrf_scores)
}

pub fn rrf_score(k: f32, rank0: usize, weight: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    ((k + rank0 as f32 + 1.).recip() * weight)
}

pub(crate) fn collect_summing_repeated<K>(scores: impl IntoIterator<Item = (K, f32)>) -> ScoreMap<K>
where
    K: Eq + Hash,
{
    scores
        .into_iter()
        .fold(ScoreMap::new(), |mut acc, (key, value)| {
            acc.entry(key).or_default().add_assign(value);
            acc
        })
}

pub(crate) fn take_highest_n_scores<K>(n: usize, scores: ScoreMap<K>) -> ScoreMap<K>
where
    K: Eq + Hash + Ord,
{
    if scores.len() <= n {
        return scores;
    }

    scores
        .into_iter()
        .sorted_unstable_by(|(k1, s1), (k2, s2)| {
            s1.total_cmp(s2).then_with(|| k1.cmp(k2)).reverse()
        })
        .take(n)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_parameters_are_used() {
        let left: ScoreMap<&'static str> = [("foo", 2.), ("bar", 1.), ("baz", 3.)].into();
        let right: ScoreMap<&'static str> = [("baz", 5.), ("dodo", 1.2)].into();
        assert_eq!(
            rrf(80., [(1., left.clone()), (1., right.clone())]),
            [
                ("foo", 1. / (80. + 2.)),
                ("bar", 1. / (80. + 3.)),
                ("baz", 1. / (80. + 1.) + 1. / (80. + 1.)),
                ("dodo", 1. / (80. + 2.)),
            ]
            .into(),
        );
        assert_eq!(
            rrf(80., [(0.2, left), (8., right)]),
            [
                ("foo", 0.2 / (80. + 2.)),
                ("bar", 0.2 / (80. + 3.)),
                ("baz", 0.2 / (80. + 1.) + 8. / (80. + 1.)),
                ("dodo", 8. / (80. + 2.)),
            ]
            .into(),
        );
    }
}
