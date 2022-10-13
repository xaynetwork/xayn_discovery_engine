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

use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    convert::identity,
    iter::once,
    sync::Arc,
    time::Duration,
};

use derivative::Derivative;
use itertools::izip;
use ndarray::{s, Array1, Array2, ArrayBase, Axis, Data, Ix, Ix2};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use xayn_discovery_engine_providers::{clean_query, Market};

use crate::{
    coi::{point::PositiveCoi, stats::compute_coi_relevances, CoiError, CoiId},
    embedding::{pairwise_cosine_similarity, Embedding},
    error::GenericError,
    utils::{nan_safe_f32_cmp, system_time_now},
};

/// A key phrase representation with a cached point.
#[derive(Clone, Debug, Derivative, Deserialize, Serialize)]
#[derivative(Eq, PartialEq)]
struct KP {
    words: String,
    #[derivative(PartialEq = "ignore")]
    point: Embedding,
}

/// A sharable key phrase.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyPhrase(Arc<KP>);

impl KeyPhrase {
    /// Creates a key phrase after validating the inputs.
    fn new(words: impl Into<String>, point: impl Into<Embedding>) -> Result<Self, CoiError> {
        let words = words.into();
        let point = point.into();

        if words.is_empty() || point.is_empty() {
            return Err(CoiError::EmptyKeyPhrase);
        }
        if !point.iter().copied().all(f32::is_finite) {
            return Err(CoiError::NonFiniteKeyPhrase(point));
        }

        Ok(Self(Arc::new(KP { words, point })))
    }

    /// Gets the words.
    pub fn words(&self) -> &str {
        &self.0.words
    }

    /// Gets the point.
    pub fn point(&self) -> &Embedding {
        &self.0.point
    }
}

impl PartialEq<&str> for KeyPhrase {
    fn eq(&self, other: &&str) -> bool {
        self.words().eq(*other)
    }
}

impl PositiveCoi {
    /// Updates the key phrases for the market.
    ///
    /// The most relevant key phrases are selected from the existing key phrases and the candidates.
    pub(super) fn update_key_phrases(
        &self,
        market: &Market,
        key_phrases: &mut KeyPhrases,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
        max_key_phrases: usize,
        gamma: f32,
    ) {
        key_phrases.update(self, market, candidates, smbert, max_key_phrases, gamma);
    }
}

/// Sorted maps from cois and markets to key phrases.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct KeyPhrases {
    // invariant: each vector of selected key phrases must be non-empty and sorted in descending relevance
    selected: HashMap<(CoiId, Market), Vec<KeyPhrase>>,
    removed: HashMap<(CoiId, Market), Vec<KeyPhrase>>,
}

impl KeyPhrases {
    /// Updates the key phrases for the positive coi and market.
    ///
    /// The most relevant key phrases are selected from the existing key phrases and the candidates.
    fn update(
        &mut self,
        coi: &PositiveCoi,
        market: &Market,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
        max_key_phrases: usize,
        gamma: f32,
    ) {
        let key_phrases = self
            .selected
            .remove(&(coi.id, market.clone()))
            .unwrap_or_default();
        let key_phrases = unify(key_phrases, candidates, smbert);
        update(
            &mut self.selected,
            coi,
            market,
            key_phrases,
            max_key_phrases,
            gamma,
        );
    }

    /// Refreshes the key phrases for the positive cois and market.
    ///
    /// If the selected key phrases are empty, then the removed key phrases are selected again.
    fn refresh(
        &mut self,
        cois: &[PositiveCoi],
        market: &Market,
        max_key_phrases: usize,
        gamma: f32,
    ) {
        if cois
            .iter()
            .all(|coi| !self.selected.contains_key(&(coi.id, market.clone())))
        {
            for coi in cois {
                // `removed` doesn't enforce the same sorting invariants as `selected`, hence we
                // have to ensure this after swapping them. we only update them once after swapping
                // to guarantee that the key phrases are fitted for the current coi point. if we
                // were to update them every time we take some, which would also be more expensive,
                // then the selection could be outdated once we swap them.
                if let Some(key_phrases) = self.removed.remove(&(coi.id, market.clone())) {
                    update(
                        &mut self.selected,
                        coi,
                        market,
                        key_phrases,
                        max_key_phrases,
                        gamma,
                    );
                }
            }
        }
    }

    /// Takes the top key phrases from the positive cois and market, sorted in descending relevance.
    pub(super) fn take(
        &mut self,
        cois: &[PositiveCoi],
        market: &Market,
        top: usize,
        horizon: Duration,
        penalty: &[f32],
        gamma: f32,
    ) -> Vec<KeyPhrase> {
        self.refresh(cois, market, penalty.len(), gamma);

        let relevances = compute_coi_relevances(cois, horizon, system_time_now());
        let mut relevances = penalize(cois, market, &self.selected, penalty, relevances);
        relevances.sort_by(|(this, _, _), (other, _, _)| nan_safe_f32_cmp(this, other).reverse());

        relevances
            .into_iter()
            .take(top)
            .map(|(_, coi_id, key_phrase)| remove(self, coi_id, market, key_phrase))
            .collect()
    }

    /// Removes all key phrases associated to the markets.
    pub(super) fn remove(&mut self, markets: &[Market]) {
        self.selected
            .retain(|(_, market), _| !markets.contains(market));
        self.removed
            .retain(|(_, market), _| !markets.contains(market));
    }
}

/// Unifies the key phrases and candidates.
fn unify(
    mut key_phrases: Vec<KeyPhrase>,
    candidates: &[String],
    smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
) -> Vec<KeyPhrase> {
    if candidates.is_empty() {
        return key_phrases;
    }

    let mut candidates = candidates
        .iter()
        .filter_map(|candidate| {
            let candidate = clean_query(candidate);
            key_phrases
                .iter()
                .all(|key_phrase| key_phrase.words() != candidate)
                .then_some(candidate)
        })
        .collect::<HashSet<_>>()
        .into_par_iter()
        .filter_map(|candidate| {
            smbert(&candidate)
                .and_then(|point| KeyPhrase::new(candidate, point).map_err(Into::into))
                .ok()
        })
        .collect();

    key_phrases.append(&mut candidates);
    key_phrases
}

/// Reduces the matrix along the axis while skipping the diagonal elements.
///
/// The elements can be prepared before they are reduced. The reduced result can be finalized
/// whereas the finalization conditially depends on whether the reduced lane overlaps with the main
/// square block of the matrix.
#[allow(clippy::needless_pass_by_value)] // pass by value needed for ArrayView
fn reduce_without_diag(
    array: ArrayBase<impl Data<Elem = f32>, Ix2>,
    axis: Axis,
    prepare: impl Fn(f32) -> f32,
    reduce: impl Fn(f32, f32) -> f32,
    finalize: impl Fn(f32, bool) -> f32,
) -> Array2<f32> {
    let reduce = &reduce;
    array
        .lanes(axis)
        .into_iter()
        .enumerate()
        .map(|(i, lane)| {
            lane.iter()
                .enumerate()
                .filter_map(|(j, element)| (i != j).then(|| prepare(*element)))
                .reduce(reduce)
                .map_or(f32::NAN, |reduced| finalize(reduced, i < lane.len()))
        })
        .collect::<Array1<_>>()
        .insert_axis(axis)
}

/// Gets the index of the maximum element.
fn argmax(iter: impl IntoIterator<Item = impl Borrow<f32>>) -> Option<Ix> {
    iter.into_iter()
        .enumerate()
        .reduce(|(arg, max), (index, element)| {
            if element.borrow() > max.borrow() {
                (index, element)
            } else {
                (arg, max)
            }
        })
        .map(|(arg, _)| arg)
}

/// Computes the pairwise, normalized similarity matrix of the key phrases.
///
/// The matrix is of shape `(key_phrases_len, key_phrases_len + 1)` where the last column holds the
/// normalized similarities between the key phrases and the coi point.
fn similarities(key_phrases: &[KeyPhrase], coi_point: &Embedding) -> Array2<f32> {
    let len = key_phrases.len();
    let similarity = pairwise_cosine_similarity(
        key_phrases
            .iter()
            .map(|key_phrase| key_phrase.point().view())
            .chain(once(coi_point.view())),
    )
    .slice_move(s![..len, ..]);
    debug_assert!(similarity.iter().copied().all(f32::is_finite));

    let min = reduce_without_diag(similarity.view(), Axis(0), identity, f32::min, |x, _| x);
    let max = reduce_without_diag(similarity.view(), Axis(0), identity, f32::max, |x, _| x);
    let normalized = (similarity - &min) / (max - min);
    let mean = reduce_without_diag(
        normalized.view(),
        Axis(0),
        identity,
        |reduced, element| reduced + element,
        #[allow(clippy::cast_precision_loss)] // small values
        |reduced, is_within_square| reduced / if is_within_square { len - 1 } else { len } as f32,
    );
    let std_dev = reduce_without_diag(
        &normalized - &mean,
        Axis(0),
        |element| element.powi(2),
        |reduced, element| reduced + element,
        #[allow(clippy::cast_precision_loss)] // small values
        |reduced, is_within_square| {
            (reduced / if is_within_square { len - 1 } else { len } as f32).sqrt()
        },
    );
    let normalized = (normalized - mean) / std_dev + 0.5;
    let normalized = normalized.mapv_into(|normalized| {
        if normalized.is_finite() {
            normalized
        } else {
            0.5
        }
    });
    debug_assert!(normalized.iter().copied().all(f32::is_finite));

    normalized
}

/// Determines which key phrases should be selected.
#[allow(clippy::needless_pass_by_value)] // pass by value needed for ArrayView
fn is_selected(
    similarity: ArrayBase<impl Data<Elem = f32>, Ix2>,
    max_key_phrases: usize,
    gamma: f32,
) -> Vec<bool> {
    let len = similarity.len_of(Axis(0));
    if len <= max_key_phrases {
        return vec![true; len];
    }

    let mut selected = vec![false; len];
    if max_key_phrases == 0 {
        return selected;
    }

    let candidate =
        argmax(similarity.slice(s![.., -1])).unwrap(/* at least one key phrase is available */);
    selected[candidate] = true;
    for _ in 0..max_key_phrases.min(len) - 1 {
        let candidate = argmax(selected.iter().zip(similarity.rows()).map(
            |(&is_selected, similarity)| {
                if is_selected {
                    f32::MIN
                } else {
                    let max = selected
                        .iter()
                        .zip(similarity)
                        .filter_map(|(is_selected, similarity)| {
                            is_selected.then(|| *similarity)
                        })
                        .reduce(f32::max)
                        .unwrap(/* at least one key phrase is selected */);
                    gamma * similarity.slice(s![-1]).into_scalar() - (1. - gamma) * max
                }
            },
        )).unwrap(/* at least one key phrase is available */);
        selected[candidate] = true;
    }

    selected
}

/// Selects the determined key phrases.
#[allow(clippy::needless_pass_by_value)] // pass by value needed for ArrayView
fn select(
    key_phrases: Vec<KeyPhrase>,
    selected: Vec<bool>,
    similarity: ArrayBase<impl Data<Elem = f32>, Ix2>,
) -> Vec<KeyPhrase> {
    debug_assert_eq!(key_phrases.len(), selected.len());
    debug_assert_eq!(key_phrases.len(), similarity.len_of(Axis(0)));

    let mut key_phrases = izip!(selected, similarity.slice(s![.., -1]), key_phrases)
        .filter_map(|(is_selected, similarity, key_phrase)| {
            is_selected.then_some((*similarity, key_phrase))
        })
        .collect::<Vec<_>>();

    key_phrases.sort_by(|(this, _), (other, _)| nan_safe_f32_cmp(this, other).reverse());

    key_phrases
        .into_iter()
        .map(|(_, key_phrase)| key_phrase)
        .collect()
}

/// Updates the key phrases and the corresponding map entry for the coi and market.
fn update(
    map: &mut HashMap<(CoiId, Market), Vec<KeyPhrase>>,
    coi: &PositiveCoi,
    market: &Market,
    key_phrases: Vec<KeyPhrase>,
    max_key_phrases: usize,
    gamma: f32,
) {
    let similarity = similarities(&key_phrases, &coi.point);
    let selected = is_selected(similarity.view(), max_key_phrases, gamma);
    let key_phrases = select(key_phrases, selected, similarity);
    if !key_phrases.is_empty() {
        map.insert((coi.id, market.clone()), key_phrases);
    }
}

/// Computes the penalized coi relevances for the key phrases.
fn penalize(
    cois: &[PositiveCoi],
    market: &Market,
    key_phrases: &HashMap<(CoiId, Market), Vec<KeyPhrase>>,
    penalty: &[f32],
    relevances: Vec<f32>,
) -> Vec<(f32, CoiId, KeyPhrase)> {
    debug_assert_eq!(cois.len(), relevances.len());
    izip!(cois, relevances)
        .filter_map(|(coi, relevance)| {
            key_phrases
                .get(&(coi.id, market.clone()))
                .map(move |key_phrases| {
                    izip!(penalty, key_phrases).map(move |(&penalty, key_phrase)| {
                        let penalized_relevance = (relevance * penalty).max(f32::MIN).min(f32::MAX);
                        (penalized_relevance, coi.id, key_phrase.clone())
                    })
                })
        })
        .flatten()
        .collect()
}

/// Removes the key phrase from the selected key phrases.
fn remove(
    key_phrases: &mut KeyPhrases,
    coi_id: CoiId,
    market: &Market,
    key_phrase: KeyPhrase,
) -> KeyPhrase {
    if let Some(selected) = key_phrases.selected.get_mut(&(coi_id, market.clone())) {
        if let Some(index) = selected
            .iter()
            .position(|kp| kp.words() == key_phrase.words())
        {
            key_phrases
                .removed
                .entry((coi_id, market.clone()))
                .or_default()
                .push(selected.remove(index));
        }
        if selected.is_empty() {
            key_phrases.selected.remove(&(coi_id, market.clone()));
        }
    }

    key_phrase
}

#[cfg(test)]
mod tests {
    use std::{mem::swap, time::Duration};

    use itertools::Itertools;
    use ndarray::arr2;
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    use crate::{
        coi::{config::Config as CoiConfig, point::tests::create_pos_cois},
        kps::config::Config as KpsConfig,
    };

    use super::*;

    impl KeyPhrases {
        pub(crate) fn new<'a, const N: usize>(
            iter: impl IntoIterator<Item = (CoiId, (&'a str, &'a str), &'a str, [f32; N])>,
        ) -> Self {
            let mut this = Self::default();
            for (coi_id, (lang_code, country_code), words, point) in iter {
                this.selected
                    .entry((coi_id, Market::new(lang_code, country_code)))
                    .or_default()
                    .push(KeyPhrase::new(words, point).unwrap());
            }
            this
        }
    }

    #[test]
    fn test_unify_empty() {
        let key_phrases = vec![];
        let candidates = [];
        let smbert = |_: &str| unreachable!();

        let key_phrases = unify(key_phrases, &candidates, smbert);
        assert!(key_phrases.is_empty());
    }

    #[test]
    fn test_unify_no_candidates() {
        let key_phrases = vec![
            KeyPhrase::new("key", [1., 0., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 0.]).unwrap(),
        ];
        let candidates = [];
        let smbert = |_: &str| unreachable!();

        let key_phrases = unify(key_phrases, &candidates, smbert);
        assert_eq!(key_phrases, ["key", "phrase"]);
    }

    #[test]
    fn test_unify_only_candidates() {
        let key_phrases = vec![];
        let candidates = ["key".into(), "phrase".into()];
        let smbert = |words: &str| match words {
            "key" => Ok([1., 0., 0.].into()),
            "phrase" => Ok([1., 1., 0.].into()),
            _ => unreachable!(),
        };

        let mut key_phrases = unify(key_phrases, &candidates, smbert);
        key_phrases.sort_by(|this, other| this.words().cmp(other.words()));
        assert_eq!(key_phrases, ["key", "phrase"]);
    }

    #[test]
    fn test_unify_duplicate() {
        let key_phrases = vec![
            KeyPhrase::new("key", [1., 0., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 0.]).unwrap(),
        ];
        let candidates = [
            "phrase".into(),
            "phrase.".into(),
            "words!".into(),
            "words?".into(),
        ];
        let smbert = |words: &str| match words {
            "phrase" => Ok([1., 1., 0.].into()),
            "words" => Ok([1., 1., 1.].into()),
            _ => unreachable!(),
        };

        let key_phrases = unify(key_phrases, &candidates, smbert);
        assert_eq!(key_phrases, ["key", "phrase", "words"]);
    }

    #[test]
    fn test_reduce_without_diag_empty() {
        let reduced = reduce_without_diag(
            Array2::default((0, 0)),
            Axis(0),
            |_| unreachable!(),
            |_, _| unreachable!(),
            |_, _| unreachable!(),
        );
        assert_eq!(reduced.shape(), [1, 0]);

        let reduced = reduce_without_diag(
            Array2::default((0, 4)),
            Axis(0),
            |_| unreachable!(),
            |_, _| unreachable!(),
            |_, _| unreachable!(),
        );
        assert_approx_eq!(f32, reduced, [[f32::NAN, f32::NAN, f32::NAN, f32::NAN]]);
    }

    #[test]
    fn test_reduce_without_diag_single() {
        let reduced = reduce_without_diag(
            Array1::range(1., 2., 1.).into_shape((1, 1)).unwrap(),
            Axis(0),
            |element| element,
            |reduced, _| reduced,
            |reduced, _| reduced,
        );
        assert_approx_eq!(f32, reduced, [[f32::NAN]]);

        let reduced = reduce_without_diag(
            Array1::range(1., 5., 1.).into_shape((2, 2)).unwrap(),
            Axis(0),
            |element| element,
            |reduced, _| reduced,
            |reduced, _| reduced,
        );
        assert_approx_eq!(f32, reduced, [[3., 2.]]);
    }

    #[test]
    fn test_reduce_without_diag_prepare() {
        let reduced = reduce_without_diag(
            Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap(),
            Axis(0),
            |element| element.powi(2),
            |reduced, _| reduced,
            |reduced, _| reduced,
        );
        assert_approx_eq!(f32, reduced, [[25., 4., 9., 16.]]);
    }

    #[test]
    fn test_reduce_without_diag_reduce() {
        let reduced = reduce_without_diag(
            Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap(),
            Axis(0),
            |element| element,
            |reduced, element| reduced + element,
            |reduced, _| reduced,
        );
        assert_approx_eq!(f32, reduced, [[14., 12., 10., 24.]]);
    }

    #[test]
    fn test_reduce_without_diag_finalize() {
        let reduced = reduce_without_diag(
            Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap(),
            Axis(0),
            |element| element,
            |reduced, _| reduced,
            |reduced, is_within_square| is_within_square.then_some(reduced).unwrap_or_default(),
        );
        assert_approx_eq!(f32, reduced, [[5., 2., 3., 0.]]);
    }

    #[test]
    fn test_reduce_without_diag_combined() {
        let array = Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap();
        let mean = reduce_without_diag(
            array.view(),
            Axis(0),
            |element| element,
            |reduced, element| reduced + element,
            |reduced, is_within_square| reduced / if is_within_square { 2. } else { 3. },
        );
        assert_approx_eq!(f32, mean, [[7., 6., 5., 8.]]);
        let stddev = reduce_without_diag(
            array - mean,
            Axis(0),
            |element| element.powi(2),
            |reduced, element| reduced + element,
            |reduced, is_within_square| {
                if is_within_square {
                    reduced / 2.
                } else {
                    reduced / 3.
                }
                .sqrt()
            },
        );
        assert_approx_eq!(f32, stddev, [[2., 4., 2., 3.265_986_4]]);
    }

    #[test]
    fn test_argmax() {
        assert!(argmax([] as [f32; 0]).is_none());
        assert_eq!(argmax([0., 0., 0.]).unwrap(), 0);
        assert_eq!(argmax([2., 0., 1.]).unwrap(), 0);
        assert_eq!(argmax([1., 2., 0.]).unwrap(), 1);
        assert_eq!(argmax([0., 1., 2.]).unwrap(), 2);
    }

    #[test]
    fn test_similarities_empty() {
        let key_phrases = [];
        let coi_point = [1., 0., 0.].into();
        let similarity = similarities(&key_phrases, &coi_point);
        assert_eq!(similarity.shape(), [0, 1]);
    }

    #[test]
    fn test_similarities_single() {
        let key_phrases = [KeyPhrase::new("key", [1., 1., 0.]).unwrap()];
        let coi_point = [1., 0., 0.].into();
        let similarity = similarities(&key_phrases, &coi_point);
        assert_approx_eq!(f32, similarity, [[0.5, 0.5]]);

        let key_phrases = [
            KeyPhrase::new("key", [1., 1., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 1.]).unwrap(),
        ];
        let similarity = similarities(&key_phrases, &coi_point);
        assert_approx_eq!(f32, similarity, [[0.5, 0.5, 1.5], [0.5, 0.5, -0.5]]);
    }

    #[test]
    fn test_similarities_multiple() {
        let key_phrases = [
            KeyPhrase::new("key", [1., 1., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 1.]).unwrap(),
            KeyPhrase::new("words", [0., 1., 1.]).unwrap(),
        ];
        let coi_point = [1., 0., 0.].into();
        let similarity = similarities(&key_phrases, &coi_point);
        assert_approx_eq!(
            f32,
            similarity,
            // the 2nd column should actually be [0.5, 0.5, 0.5], but the similarity computation
            // between 2/sqrt(2)/sqrt(3) and 2/sqrt(3)/sqrt(2) has numerical rounding errors which
            // carries over to the normalization. this doesn't matter though, as the result can just
            // be interpreted as a "random" choice, caused by the numerical precision issue, between
            // the two equally likely options.
            [
                [2.659_591, 1.5, -0.5, 1.407_614_8],
                [1.5, 6_157_353.5, 1.5, 0.985_435],
                [-0.5, -0.5, 2.659_591, -0.893_049_9],
            ],
            epsilon = 1e-5,
        );
    }

    #[test]
    fn test_is_selected_empty() {
        let selected = is_selected(Array2::default((0, 1)).view(), 0, 0.9);
        assert!(selected.is_empty());

        let selected = is_selected(Array2::default((0, 1)).view(), 3, 0.9);
        assert!(selected.is_empty());
    }

    #[test]
    fn test_is_selected_all() {
        let selected = is_selected(Array2::default((2, 3)).view(), 0, 0.9);
        assert_eq!(selected, [false, false]);

        let selected = is_selected(Array2::default((2, 3)).view(), 3, 0.9);
        assert_eq!(selected, [true, true]);
    }

    #[test]
    fn test_is_selected_multiple() {
        let similarity = arr2(&[[1., 2., 3., 1.], [3., 2., 1., 0.], [1., 1., 1., 2.]]);
        let selected = is_selected(similarity.view(), 2, 0.);
        assert_eq!(selected, [false, true, true]);

        let selected = is_selected(similarity.view(), 2, 0.4);
        assert_eq!(selected, [false, true, true]);

        let selected = is_selected(similarity.view(), 2, 0.9);
        assert_eq!(selected, [true, false, true]);

        let selected = is_selected(similarity, 2, 1.);
        assert_eq!(selected, [true, false, true]);
    }

    #[test]
    fn test_select_empty() {
        let selection = select(vec![], vec![], Array2::default((0, 1)));
        assert!(selection.is_empty());
    }

    #[test]
    fn test_select_all() {
        let key_phrases = vec![
            KeyPhrase::new("key", [1., 1., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 1.]).unwrap(),
            KeyPhrase::new("words", [0., 1., 1.]).unwrap(),
        ];
        let similarity = Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap();

        let selection = select(
            key_phrases.clone(),
            vec![false, false, false],
            similarity.view(),
        );
        assert!(selection.is_empty());

        let selection = select(key_phrases, vec![true, true, true], similarity);
        assert_eq!(selection, ["words", "phrase", "key"]);
    }

    #[test]
    fn test_select_multiple() {
        let key_phrases = vec![
            KeyPhrase::new("key", [1., 1., 0.]).unwrap(),
            KeyPhrase::new("phrase", [1., 1., 1.]).unwrap(),
            KeyPhrase::new("words", [0., 1., 1.]).unwrap(),
        ];
        let similarity = Array1::range(1., 13., 1.).into_shape((3, 4)).unwrap();

        let selection = select(
            key_phrases.clone(),
            vec![true, true, false],
            similarity.view(),
        );
        assert_eq!(selection, ["phrase", "key"]);

        let selection = select(
            key_phrases.clone(),
            vec![true, false, true],
            similarity.view(),
        );
        assert_eq!(selection, ["words", "key"]);

        let selection = select(key_phrases, vec![false, true, true], similarity);
        assert_eq!(selection, ["words", "phrase"]);
    }

    #[test]
    fn test_update_key_phrases_empty() {
        let mut key_phrases = KeyPhrases::default();
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let market = Market::new("aa", "AA");
        let candidates = [];
        let smbert = |_: &str| unreachable!();
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_no_candidates() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 0.]),
            (cois[0].id, ("aa", "AA"), "phrase", [1., 1., 1.]),
        ]);
        let market = Market::new("aa", "AA");
        let candidates = [];
        let smbert = |_: &str| unreachable!();
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["key", "phrase"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_only_candidates() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::default();
        let market = Market::new("aa", "AA");
        let candidates = ["key".into(), "phrase".into()];
        let smbert = |words: &str| match words {
            "key" => Ok([1., 1., 0.].into()),
            "phrase" => Ok([1., 1., 1.].into()),
            _ => unreachable!(),
        };
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["key", "phrase"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_candidates_words_cleaned() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::default();
        let market = Market::new("aa", "AA");
        let candidates = ["  a  !@#$%  b  ".into()];
        let smbert = |_: &str| Ok([1., 1., 0.].into());
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&(cois[0].id, market)], ["a b"]);
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_max() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [1., 1., 0.]),
        ]);
        let market = Market::new("aa", "AA");
        let candidates = ["test".into(), "words".into()];
        let smbert = |words: &str| match words {
            "test" => Ok([1., 1., 1.].into()),
            "words" => Ok([2., 1., 0.].into()),
            _ => unreachable!(),
        };
        let config = KpsConfig::default();
        assert_eq!(config.max_key_phrases(), 3);

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["words", "key", "phrase"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_duplicate() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new([(cois[0].id, ("aa", "AA"), "key", [1., 1., 0.])]);
        let market = Market::new("aa", "AA");
        let candidates = ["phrase".into(), "phrase".into()];
        let smbert = |words: &str| match words {
            "phrase" => Ok([1., 1., 1.].into()),
            _ => unreachable!(),
        };
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["key", "phrase"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [2., 1., 0.]),
            (cois[0].id, ("bb", "BB"), "phrase", [1., 1., 0.]),
        ]);
        let market = Market::new("aa", "AA");
        let candidates = ["words".into()];
        let smbert = |words: &str| match words {
            "words" => Ok([3., 1., 0.].into()),
            _ => unreachable!(),
        };
        let config = KpsConfig::default();

        key_phrases.update(
            &cois[0],
            &market,
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), 2);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["words", "key"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("bb", "BB"))],
            ["phrase"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_refresh_key_phrases_empty_cois() {
        let cois = create_pos_cois(&[] as &[[f32; 0]]);
        let coi_id = CoiId::mocked(1);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([(coi_id, ("aa", "AA"), "key", [1., 1., 1.])]);
        swap(&mut key_phrases.selected, &mut key_phrases.removed);
        let config = KpsConfig::default();

        key_phrases.refresh(&cois, &market, config.max_key_phrases(), config.gamma());
        assert!(key_phrases.selected.is_empty());
        assert_eq!(key_phrases.removed.len(), 1);
        assert_eq!(key_phrases.removed[&(coi_id, market)], ["key"]);
    }

    #[test]
    fn test_refresh_key_phrases_empty_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let market = Market::new("bb", "BB");
        let mut key_phrases = KeyPhrases::new([(cois[0].id, ("aa", "AA"), "key", [1., 1., 1.])]);
        swap(&mut key_phrases.selected, &mut key_phrases.removed);
        let config = KpsConfig::default();

        key_phrases.refresh(&cois, &market, config.max_key_phrases(), config.gamma());
        assert!(key_phrases.selected.is_empty());
        assert_eq!(key_phrases.removed.len(), 1);
        assert_eq!(
            key_phrases.removed[&(cois[0].id, Market::new("aa", "AA"))],
            ["key"],
        );
    }

    #[test]
    fn test_refresh_key_phrases_empty_key_phrases() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::default();
        let config = KpsConfig::default();

        key_phrases.refresh(&cois, &market, config.max_key_phrases(), config.gamma());
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_refresh_key_phrases_too_many_key_phrases() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.]]);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "words", [3., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "and", [4., 1., 1.]),
            (cois[0].id, ("bb", "BB"), "more", [6., 1., 1.]),
            (cois[0].id, ("bb", "BB"), "stuff", [5., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "still", [1., 3., 1.]),
            (cois[1].id, ("aa", "AA"), "not", [1., 2., 1.]),
            (cois[1].id, ("bb", "BB"), "enough", [1., 1., 1.]),
        ]);
        swap(&mut key_phrases.selected, &mut key_phrases.removed);
        let config = KpsConfig::default();
        assert_eq!(config.max_key_phrases(), 3);

        key_phrases.refresh(&cois, &market, config.max_key_phrases(), config.gamma());
        assert_eq!(key_phrases.selected.len(), 2);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market.clone())],
            ["and", "words", "phrase"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[1].id, market.clone())],
            ["still", "not"],
        );
        assert_eq!(key_phrases.removed.len(), 2);
        assert_eq!(
            key_phrases.removed[&(cois[0].id, Market::new("bb", "BB"))],
            ["more", "stuff"],
        );
        assert_eq!(
            key_phrases.removed[&(cois[1].id, Market::new("bb", "BB"))],
            ["enough"],
        );

        key_phrases.selected.remove(&(cois[1].id, market.clone()));
        key_phrases
            .removed
            .remove(&(cois[1].id, Market::new("bb", "BB")));
        key_phrases.refresh(&cois, &market, config.max_key_phrases(), config.gamma());
        assert_eq!(key_phrases.selected.len(), 1);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market)],
            ["and", "words", "phrase"],
        );
        assert_eq!(key_phrases.removed.len(), 1);
        assert_eq!(
            key_phrases.removed[&(cois[0].id, Market::new("bb", "BB"))],
            ["more", "stuff"],
        );
    }

    #[test]
    fn test_penalize() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let market = Market::new("aa", "AA");
        let key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [3., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "words", [1., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "and", [1., 6., 1.]),
            (cois[1].id, ("aa", "AA"), "more", [1., 5., 1.]),
            (cois[1].id, ("bb", "BB"), "stuff", [1., 4., 1.]),
            (cois[2].id, ("aa", "AA"), "still", [1., 1., 9.]),
            (cois[2].id, ("bb", "BB"), "not", [1., 1., 8.]),
            (cois[2].id, ("bb", "BB"), "enough", [1., 1., 7.]),
        ]);
        let penalty = [1., 0.8, 0.6];
        let relevances = vec![1., 2., 3.];

        let (relevances, coi_ids, key_phrases) =
            penalize(&cois, &market, &key_phrases.selected, &penalty, relevances)
                .into_iter()
                .multiunzip::<(Vec<_>, Vec<_>, Vec<_>)>();
        assert_approx_eq!(f32, relevances, [1., 0.8, 0.6, 2., 1.6, 3.]);
        assert_eq!(
            coi_ids,
            [cois[0].id, cois[0].id, cois[0].id, cois[1].id, cois[1].id, cois[2].id],
        );
        assert_eq!(
            key_phrases,
            ["key", "phrase", "words", "and", "more", "still"],
        );
    }

    #[test]
    fn test_remove_missing() {
        let coi_id = CoiId::mocked(1);
        let market = Market::new("aa", "AA");
        let key_phrase = KeyPhrase::new("key", [1., 1., 1.]).unwrap();
        let mut key_phrases = KeyPhrases::default();

        let key_phrase = remove(&mut key_phrases, coi_id, &market, key_phrase);
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
        assert_eq!(key_phrase, "key");
    }

    #[test]
    fn test_remove_existent() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.]]);
        let market = Market::new("aa", "AA");
        let key_phrase = KeyPhrase::new("key", [1., 1., 1.]).unwrap();
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [1., 1., 0.]),
            (cois[1].id, ("aa", "AA"), "words", [0., 1., 1.]),
        ]);

        let key_phrase = remove(&mut key_phrases, cois[0].id, &market, key_phrase);
        assert_eq!(key_phrases.selected.len(), 2);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market.clone())],
            ["phrase"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[1].id, market.clone())],
            ["words"],
        );
        assert_eq!(key_phrases.removed.len(), 1);
        assert_eq!(key_phrases.removed[&(cois[0].id, market.clone())], ["key"]);
        assert_eq!(key_phrase, "key");

        let key_phrase = KeyPhrase::new("words", [0., 1., 1.]).unwrap();

        let key_phrase = remove(&mut key_phrases, cois[1].id, &market, key_phrase);
        assert_eq!(key_phrases.selected.len(), 1);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, market.clone())],
            ["phrase"],
        );
        assert_eq!(key_phrases.removed.len(), 2);
        assert_eq!(key_phrases.removed[&(cois[0].id, market.clone())], ["key"]);
        assert_eq!(
            key_phrases.removed[&(cois[1].id, market.clone())],
            ["words"],
        );
        assert_eq!(key_phrase, "words");
    }

    #[test]
    fn test_take_key_phrases_empty_cois() {
        let cois = create_pos_cois(&[] as &[[f32; 0]]);
        let coi_id = CoiId::mocked(1);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([(coi_id, ("aa", "AA"), "key", [1., 1., 1.])]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert_eq!(key_phrases.selected.len(), 1);
        assert_eq!(key_phrases.selected[&(coi_id, market)], ["key"]);
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_empty_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let market = Market::new("bb", "BB");
        let mut key_phrases = KeyPhrases::new([(cois[0].id, ("aa", "AA"), "key", [1., 1., 1.])]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert_eq!(key_phrases.selected.len(), 1);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("aa", "AA"))],
            ["key"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_empty_key_phrases() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::default();
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_zero() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[2].id, ("aa", "AA"), "words", [3., 1., 1.]),
        ]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            0,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&(cois[0].id, market.clone())], ["key"]);
        assert_eq!(
            key_phrases.selected[&(cois[1].id, market.clone())],
            ["phrase"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[2].id, market.clone())],
            ["words"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_all() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        cois[0].log_time(Duration::from_secs(11));
        cois[1].log_time(Duration::from_secs(12));
        cois[2].log_time(Duration::from_secs(13));
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [3., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "words", [1., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "and", [1., 6., 1.]),
            (cois[1].id, ("aa", "AA"), "more", [1., 5., 1.]),
            (cois[1].id, ("aa", "AA"), "stuff", [1., 4., 1.]),
            (cois[2].id, ("aa", "AA"), "still", [1., 1., 9.]),
            (cois[2].id, ("aa", "AA"), "not", [1., 1., 8.]),
            (cois[2].id, ("aa", "AA"), "enough", [1., 1., 7.]),
        ]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert_eq!(top_key_phrases.len(), 9);
        assert_eq!(
            top_key_phrases,
            ["still", "and", "key", "not", "more", "phrase", "enough", "stuff", "words"],
        );
        assert!(key_phrases.selected.is_empty());
        assert_eq!(key_phrases.removed.len(), 3);
        assert_eq!(
            key_phrases.removed[&(cois[0].id, market.clone())],
            ["key", "phrase", "words"],
        );
        assert_eq!(
            key_phrases.removed[&(cois[1].id, market.clone())],
            ["and", "more", "stuff"],
        );
        assert_eq!(
            key_phrases.removed[&(cois[2].id, market.clone())],
            ["still", "not", "enough"],
        );
    }

    #[test]
    fn test_take_key_phrases_market() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        cois[0].log_time(Duration::from_secs(11));
        cois[1].log_time(Duration::from_secs(12));
        cois[2].log_time(Duration::from_secs(13));
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [3., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "words", [1., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "and", [1., 6., 1.]),
            (cois[1].id, ("aa", "AA"), "more", [1., 5., 1.]),
            (cois[1].id, ("bb", "BB"), "stuff", [1., 4., 1.]),
            (cois[2].id, ("aa", "AA"), "still", [1., 1., 9.]),
            (cois[2].id, ("bb", "BB"), "not", [1., 1., 8.]),
            (cois[2].id, ("bb", "BB"), "enough", [1., 1., 7.]),
        ]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert_eq!(top_key_phrases.len(), 6);
        assert_eq!(
            top_key_phrases,
            ["still", "and", "key", "more", "phrase", "words"],
        );
        assert_eq!(key_phrases.selected.len(), 2);
        assert_eq!(
            key_phrases.selected[&(cois[1].id, Market::new("bb", "BB"))],
            ["stuff"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[2].id, Market::new("bb", "BB"))],
            ["not", "enough"],
        );
        assert_eq!(key_phrases.removed.len(), 3);
        assert_eq!(
            key_phrases.removed[&(cois[0].id, market.clone())],
            ["key", "phrase", "words"],
        );
        assert_eq!(
            key_phrases.removed[&(cois[1].id, market.clone())],
            ["and", "more"],
        );
        assert_eq!(
            key_phrases.removed[&(cois[2].id, market.clone())],
            ["still"],
        );
    }

    #[test]
    fn test_take_key_phrases_refresh_if_empty() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        cois[0].log_time(Duration::from_secs(11));
        cois[1].log_time(Duration::from_secs(12));
        cois[2].log_time(Duration::from_secs(13));
        let market = Market::new("aa", "AA");
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [3., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "phrase", [2., 1., 1.]),
            (cois[0].id, ("aa", "AA"), "words", [1., 1., 1.]),
            (cois[1].id, ("aa", "AA"), "and", [1., 6., 1.]),
            (cois[1].id, ("aa", "AA"), "more", [1., 5., 1.]),
            (cois[1].id, ("bb", "BB"), "stuff", [1., 4., 1.]),
            (cois[2].id, ("aa", "AA"), "still", [1., 1., 9.]),
            (cois[2].id, ("bb", "BB"), "not", [1., 1., 8.]),
            (cois[2].id, ("bb", "BB"), "enough", [1., 1., 7.]),
        ]);
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let top_key_phrases_first = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        let top_key_phrases_second = key_phrases.take(
            &cois,
            &market,
            usize::MAX,
            coi_config.horizon(),
            kps_config.penalty(),
            kps_config.gamma(),
        );
        assert_eq!(top_key_phrases_first, top_key_phrases_second);
    }

    #[test]
    fn test_remove_key_phrases_empty_key_phrases() {
        let markets = [Market::new("aa", "AA")];
        let mut key_phrases = KeyPhrases::default();

        key_phrases.remove(&markets);
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_remove_key_phrases_empty_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let markets = [];
        let mut key_phrases = KeyPhrases::new([(cois[0].id, ("aa", "AA"), "key", [1., 1., 1.])]);

        key_phrases.remove(&markets);
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("aa", "AA"))],
            ["key"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_remove_key_phrases_same_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.]]);
        let markets = [Market::new("aa", "AA"), Market::new("bb", "BB")];
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[0].id, ("bb", "BB"), "phrase", [1., 0., 1.]),
            (cois[1].id, ("bb", "BB"), "words", [1., 1., 0.]),
        ]);

        key_phrases.remove(&markets);
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_remove_key_phrases_different_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.]]);
        let markets = [Market::new("cc", "CC")];
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[0].id, ("bb", "BB"), "phrase", [1., 0., 1.]),
            (cois[1].id, ("bb", "BB"), "words", [1., 1., 0.]),
        ]);

        key_phrases.remove(&markets);
        assert_eq!(key_phrases.selected.len(), 3);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("aa", "AA"))],
            ["key"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("bb", "BB"))],
            ["phrase"],
        );
        assert_eq!(
            key_phrases.selected[&(cois[1].id, Market::new("bb", "BB"))],
            ["words"],
        );
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_remove_key_phrases_mixed_markets() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.]]);
        let markets = [Market::new("bb", "BB"), Market::new("cc", "CC")];
        let mut key_phrases = KeyPhrases::new([
            (cois[0].id, ("aa", "AA"), "key", [1., 1., 1.]),
            (cois[0].id, ("bb", "BB"), "phrase", [1., 0., 1.]),
            (cois[1].id, ("bb", "BB"), "words", [1., 1., 0.]),
        ]);

        key_phrases.remove(&markets);
        assert_eq!(key_phrases.selected.len(), 1);
        assert_eq!(
            key_phrases.selected[&(cois[0].id, Market::new("aa", "AA"))],
            ["key"],
        );
        assert!(key_phrases.removed.is_empty());
    }
}
