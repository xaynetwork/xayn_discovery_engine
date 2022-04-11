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
    mem::swap,
    sync::Arc,
    time::Duration,
};

use derivative::Derivative;
use itertools::izip;
use ndarray::{s, Array1, Array2, ArrayBase, ArrayView2, Axis, Data, Ix, Ix2};
#[cfg(feature = "multithreaded")]
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::{
    coi::{point::PositiveCoi, stats::compute_coi_relevances, CoiError, CoiId},
    embedding::utils::{pairwise_cosine_similarity, Embedding},
    error::Error,
    utils::{nan_safe_f32_cmp, system_time_now},
};

#[derive(Clone, Debug, Derivative, Deserialize, Serialize)]
#[derivative(Eq, PartialEq)]
struct KP {
    words: String,
    #[derivative(PartialEq = "ignore")]
    point: Embedding,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyPhrase(Arc<KP>);

impl KeyPhrase {
    pub(super) fn new(
        words: impl Into<String>,
        point: impl Into<Embedding>,
    ) -> Result<Self, CoiError> {
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

    pub fn words(&self) -> &str {
        &self.0.words
    }

    pub fn point(&self) -> &Embedding {
        &self.0.point
    }
}

impl PositiveCoi {
    pub(super) fn update_key_phrases(
        &self,
        key_phrases: &mut KeyPhrases,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, Error> + Sync,
        max_key_phrases: usize,
        gamma: f32,
    ) {
        key_phrases.update(self, candidates, smbert, max_key_phrases, gamma);
    }
}

/// Sorted maps from cois to key phrases.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct KeyPhrases {
    // invariant: each vector of selected key phrases must be sorted in descending relevance
    selected: HashMap<CoiId, Vec<KeyPhrase>>,
    removed: HashMap<CoiId, Vec<KeyPhrase>>,
}

impl KeyPhrases {
    /// Updates the key phrases for the positive coi.
    ///
    /// The most relevant key phrases are selected from the set of key phrases of the coi and the
    /// candidates.
    fn update(
        &mut self,
        coi: &PositiveCoi,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, Error> + Sync,
        max_key_phrases: usize,
        gamma: f32,
    ) {
        let key_phrases = self.selected.remove(&coi.id).unwrap_or_default();
        let key_phrases = unify(key_phrases, candidates, smbert);
        let similarity = similarities(&key_phrases, &coi.point);
        let selected = is_selected(similarity.view(), max_key_phrases, gamma);
        let key_phrases = select(key_phrases, selected, similarity);
        if !key_phrases.is_empty() {
            self.selected.insert(coi.id, key_phrases);
        }
    }

    /// Takes the top key phrases from the positive cois, sorted in descending relevance.
    pub(super) fn take(
        &mut self,
        cois: &[PositiveCoi],
        top: usize,
        horizon: Duration,
        penalty: &[f32],
        gamma: f32,
    ) -> Vec<KeyPhrase> {
        if self.selected.is_empty() {
            swap(&mut self.selected, &mut self.removed);
            let max_key_phrases = penalty.len();
            for coi in cois {
                self.update(coi, &[], |_| unreachable!(), max_key_phrases, gamma);
            }
        }

        let relevances = compute_coi_relevances(cois, horizon, system_time_now());
        let mut relevances = izip!(cois, relevances)
            .filter_map(|(coi, relevance)| {
                self.selected.get(&coi.id).map(move |key_phrases| {
                    izip!(penalty, key_phrases).map(move |(&penalty, key_phrase)| {
                        let penalized_relevance = (relevance * penalty).max(f32::MIN).min(f32::MAX);
                        (penalized_relevance, coi.id, key_phrase.clone())
                    })
                })
            })
            .flatten()
            .collect::<Vec<_>>();
        relevances.sort_by(|(this, _, _), (other, _, _)| nan_safe_f32_cmp(this, other).reverse());

        relevances
            .into_iter()
            .take(top)
            .map(|(_, coi_id, key_phrase)| {
                if let Some(key_phrases) = self.selected.get_mut(&coi_id) {
                    if let Some(index) = key_phrases
                        .iter()
                        .position(|kp| kp.words() == key_phrase.words())
                    {
                        self.removed
                            .entry(coi_id)
                            .or_default()
                            .push(key_phrases.remove(index));
                    }
                    if key_phrases.is_empty() {
                        self.selected.remove(&coi_id);
                    }
                }
                key_phrase
            })
            .collect()
    }
}

/// Unifies the key phrases and candidates.
fn unify(
    key_phrases: Vec<KeyPhrase>,
    candidates: &[String],
    smbert: impl Fn(&str) -> Result<Embedding, Error> + Sync,
) -> Vec<KeyPhrase> {
    #[cfg(not(feature = "multithreaded"))]
    let candidates = candidates.iter();
    #[cfg(feature = "multithreaded")]
    let candidates = candidates.into_par_iter();

    let candidates = candidates
        .filter(|&candidate| {
            key_phrases
                .iter()
                .all(|key_phrase| key_phrase.words() != candidate)
        })
        .map(clean_key_phrase)
        .collect::<HashSet<_>>()
        .into_iter()
        .filter_map(|candidate| {
            smbert(&candidate)
                .and_then(|point| KeyPhrase::new(candidate, point).map_err(|e| e.into()))
                .ok()
        });

    key_phrases.into_iter().chain(candidates).collect()
}

/// Reduces the matrix along the axis while skipping the diagonal elements.
///
/// The elements can be prepared before they are reduced. The reduced result can be finalized
/// whereas the finalization conditially depends on whether the reduced lane is part of the main
/// square block of the matrix.
fn reduce_without_diag<S, P, R, F>(
    a: ArrayBase<S, Ix2>,
    axis: Axis,
    prepare: P,
    reduce: R,
    finalize: F,
) -> Array2<f32>
where
    S: Data<Elem = f32>,
    P: Fn(f32) -> f32,
    R: Fn(f32, f32) -> f32,
    F: Fn(f32, bool) -> f32,
{
    a.lanes(axis)
        .into_iter()
        .enumerate()
        .map(|(i, lane)| {
            lane.iter()
                .enumerate()
                .filter_map(|(j, element)| (i != j).then(|| prepare(*element)))
                .reduce(|x, y| reduce(x, y))
                .map(|reduced| finalize(reduced, i < lane.len()))
                .unwrap_or_default()
        })
        .collect::<Array1<_>>()
        .insert_axis(axis)
}

/// Gets the index of the maximum element.
fn argmax<I, F>(iter: I) -> Option<Ix>
where
    I: IntoIterator<Item = F>,
    F: Borrow<f32>,
{
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
        |reduced, is_within_square| {
            reduced / is_within_square.then(|| len - 1).unwrap_or(len) as f32
        },
    );
    let std_dev = reduce_without_diag(
        &normalized - &mean,
        Axis(0),
        |element| element.powi(2),
        |reduced, element| reduced + element,
        |reduced, is_within_square| {
            (reduced / is_within_square.then(|| len - 1).unwrap_or(len) as f32).sqrt()
        },
    );
    let normalized = (normalized - mean) / std_dev + 0.5;
    let normalized = normalized
        .mapv_into(|normalized| normalized.is_finite().then(|| normalized).unwrap_or(0.5));
    debug_assert!(normalized.iter().copied().all(f32::is_finite));

    normalized
}

/// Determines which key phrases should be selected.
fn is_selected(similarity: ArrayView2<f32>, max_key_phrases: usize, gamma: f32) -> Vec<bool> {
    let len = similarity.len_of(Axis(0));
    if len <= max_key_phrases {
        return vec![true; len];
    }

    let candidate =
        argmax(similarity.slice(s![.., -1])).unwrap(/* at least one key phrase is available */);
    let mut selected = vec![false; len];
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
fn select(
    key_phrases: Vec<KeyPhrase>,
    selected: Vec<bool>,
    similarity: Array2<f32>,
) -> Vec<KeyPhrase> {
    let mut key_phrases = izip!(selected, similarity.slice_move(s![.., -1]), key_phrases)
        .filter_map(|(is_selected, similarity, key_phrase)| {
            is_selected.then(|| (similarity, key_phrase))
        })
        .collect::<Vec<_>>();

    key_phrases.sort_by(|(this, _), (other, _)| nan_safe_f32_cmp(this, other).reverse());

    key_phrases
        .into_iter()
        .map(|(_, key_phrase)| key_phrase)
        .collect()
}

/// Clean a key phrase from symbols and multiple spaces.
fn clean_key_phrase(key_phrase: impl AsRef<str>) -> String {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        // match any sequence of symbols and spaces that can follow
        static ref SYMBOLS: Regex = Regex::new(r"[\p{Symbol}\p{Punctuation}]+\p{Separator}*").unwrap();
        // match any sequence spaces
        static ref SEPARATORS: Regex = Regex::new(r"\p{Separator}+").unwrap();
    }

    // we replace a symbol with a space
    let no_symbols = SYMBOLS.replace_all(key_phrase.as_ref(), " ");
    // we collapse sequence of spaces to only one
    SEPARATORS.replace_all(&no_symbols, " ").trim().to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ndarray::arr1;

    use crate::coi::{config::Config, utils::tests::create_pos_cois};

    use super::*;

    impl KeyPhrases {
        pub(crate) fn new<const N: usize>(
            coi_ids: [CoiId; N],
            key_phrases: [KeyPhrase; N],
        ) -> Self {
            let mut this = Self::default();
            for (coi_id, key_phrase) in izip!(coi_ids, key_phrases) {
                this.selected.entry(coi_id).or_default().push(key_phrase);
            }
            this
        }
    }

    #[test]
    fn test_update_key_phrases_empty() {
        let mut key_phrases = KeyPhrases::default();
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let candidates = &[];
        let smbert = |_: &str| unreachable!();
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            candidates,
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
        let mut key_phrases = KeyPhrases::new(
            [cois[0].id; 2],
            [
                KeyPhrase::new("key", arr1(&[1., 1., 0.])).unwrap(),
                KeyPhrase::new("phrase", arr1(&[1., 1., 1.])).unwrap(),
            ],
        );
        let candidates = &[];
        let smbert = |_: &str| unreachable!();
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&cois[0].id].len(), 2);
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "key");
        assert_eq!(key_phrases.selected[&cois[0].id][1].words(), "phrase");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_only_candidates() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::default();
        let candidates = ["key".into(), "phrase".into()];
        let smbert = |words: &str| match words {
            "key" => Ok(arr1(&[1., 1., 0.]).into()),
            "phrase" => Ok(arr1(&[1., 1., 1.]).into()),
            _ => unreachable!(),
        };
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&cois[0].id].len(), 2);
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "key");
        assert_eq!(key_phrases.selected[&cois[0].id][1].words(), "phrase");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_candidates_words_cleaned() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::default();
        let candidates = ["  a  !@#$%  b  ".into()];

        let smbert = |_: &str| Ok(arr1(&[1., 1., 0.]).into());
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&cois[0].id].len(), 1);
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "a b");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_max() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new(
            [cois[0].id; 2],
            [
                KeyPhrase::new("key", arr1(&[2., 1., 1.])).unwrap(),
                KeyPhrase::new("phrase", arr1(&[1., 1., 0.])).unwrap(),
            ],
        );
        let candidates = ["test".into(), "words".into()];
        let smbert = |words: &str| match words {
            "test" => Ok(arr1(&[1., 1., 1.]).into()),
            "words" => Ok(arr1(&[2., 1., 0.]).into()),
            _ => unreachable!(),
        };
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(
            key_phrases.selected[&cois[0].id].len(),
            config.max_key_phrases(),
        );
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "words");
        assert_eq!(key_phrases.selected[&cois[0].id][1].words(), "key");
        assert_eq!(key_phrases.selected[&cois[0].id][2].words(), "phrase");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_update_key_phrases_duplicate() {
        let cois = create_pos_cois(&[[1., 0., 0.]]);
        let mut key_phrases = KeyPhrases::new(
            [cois[0].id],
            [KeyPhrase::new("key", arr1(&[1., 1., 0.])).unwrap()],
        );
        let candidates = ["phrase".into(), "phrase".into()];
        let smbert = |words: &str| match words {
            "phrase" => Ok(arr1(&[1., 1., 1.]).into()),
            _ => unreachable!(),
        };
        let config = Config::default();

        key_phrases.update(
            &cois[0],
            &candidates,
            smbert,
            config.max_key_phrases(),
            config.gamma(),
        );
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&cois[0].id].len(), 2);
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "key");
        assert_eq!(key_phrases.selected[&cois[0].id][1].words(), "phrase");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_empty_cois() {
        let cois = create_pos_cois(&[] as &[[f32; 0]]);
        let mut key_phrases = KeyPhrases::default();
        let config = Config::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            usize::MAX,
            config.horizon(),
            config.penalty(),
            config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_empty_key_phrases() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let mut key_phrases = KeyPhrases::default();
        let config = Config::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            usize::MAX,
            config.horizon(),
            config.penalty(),
            config.gamma(),
        );
        assert!(top_key_phrases.is_empty());
        assert!(key_phrases.selected.is_empty());
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_zero() {
        let cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        let mut key_phrases = KeyPhrases::new(
            [cois[0].id, cois[1].id, cois[2].id],
            [
                KeyPhrase::new("key", arr1(&[1., 1., 1.])).unwrap(),
                KeyPhrase::new("phrase", arr1(&[2., 1., 1.])).unwrap(),
                KeyPhrase::new("words", arr1(&[3., 1., 1.])).unwrap(),
            ],
        );
        let config = Config::default();

        let top_key_phrases =
            key_phrases.take(&cois, 0, config.horizon(), config.penalty(), config.gamma());
        assert!(top_key_phrases.is_empty());
        assert_eq!(key_phrases.selected.len(), cois.len());
        assert_eq!(key_phrases.selected[&cois[0].id].len(), 1);
        assert_eq!(key_phrases.selected[&cois[0].id][0].words(), "key");
        assert_eq!(key_phrases.selected[&cois[1].id].len(), 1);
        assert_eq!(key_phrases.selected[&cois[1].id][0].words(), "phrase");
        assert_eq!(key_phrases.selected[&cois[2].id].len(), 1);
        assert_eq!(key_phrases.selected[&cois[2].id][0].words(), "words");
        assert!(key_phrases.removed.is_empty());
    }

    #[test]
    fn test_take_key_phrases_all() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        cois[0].log_time(Duration::from_secs(11));
        cois[1].log_time(Duration::from_secs(12));
        cois[2].log_time(Duration::from_secs(13));
        let mut key_phrases = KeyPhrases::new(
            [
                cois[0].id, cois[0].id, cois[0].id, cois[1].id, cois[1].id, cois[1].id, cois[2].id,
                cois[2].id, cois[2].id,
            ],
            [
                KeyPhrase::new("key", arr1(&[3., 1., 1.])).unwrap(),
                KeyPhrase::new("phrase", arr1(&[2., 1., 1.])).unwrap(),
                KeyPhrase::new("words", arr1(&[1., 1., 1.])).unwrap(),
                KeyPhrase::new("and", arr1(&[1., 6., 1.])).unwrap(),
                KeyPhrase::new("more", arr1(&[1., 5., 1.])).unwrap(),
                KeyPhrase::new("stuff", arr1(&[1., 4., 1.])).unwrap(),
                KeyPhrase::new("still", arr1(&[1., 1., 9.])).unwrap(),
                KeyPhrase::new("not", arr1(&[1., 1., 8.])).unwrap(),
                KeyPhrase::new("enough", arr1(&[1., 1., 7.])).unwrap(),
            ],
        );
        let config = Config::default();

        let top_key_phrases = key_phrases.take(
            &cois,
            usize::MAX,
            config.horizon(),
            config.penalty(),
            config.gamma(),
        );
        assert_eq!(top_key_phrases.len(), 9);
        assert_eq!(top_key_phrases[0].words(), "still");
        assert_eq!(top_key_phrases[1].words(), "and");
        assert_eq!(top_key_phrases[2].words(), "key");
        assert_eq!(top_key_phrases[3].words(), "not");
        assert_eq!(top_key_phrases[4].words(), "more");
        assert_eq!(top_key_phrases[5].words(), "phrase");
        assert_eq!(top_key_phrases[6].words(), "enough");
        assert_eq!(top_key_phrases[7].words(), "stuff");
        assert_eq!(top_key_phrases[8].words(), "words");
        assert!(key_phrases.selected.is_empty());
        assert_eq!(key_phrases.removed.len(), 3);
        assert_eq!(key_phrases.removed[&cois[0].id].len(), 3);
        assert_eq!(key_phrases.removed[&cois[0].id][0].words(), "key");
        assert_eq!(key_phrases.removed[&cois[0].id][1].words(), "phrase");
        assert_eq!(key_phrases.removed[&cois[0].id][2].words(), "words");
        assert_eq!(key_phrases.removed[&cois[1].id].len(), 3);
        assert_eq!(key_phrases.removed[&cois[1].id][0].words(), "and");
        assert_eq!(key_phrases.removed[&cois[1].id][1].words(), "more");
        assert_eq!(key_phrases.removed[&cois[1].id][2].words(), "stuff");
        assert_eq!(key_phrases.removed[&cois[2].id].len(), 3);
        assert_eq!(key_phrases.removed[&cois[2].id][0].words(), "still");
        assert_eq!(key_phrases.removed[&cois[2].id][1].words(), "not");
        assert_eq!(key_phrases.removed[&cois[2].id][2].words(), "enough");
    }

    #[test]
    fn test_take_key_phrases_restore_key_phrases_if_empty() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]);
        cois[0].log_time(Duration::from_secs(11));
        cois[1].log_time(Duration::from_secs(12));
        cois[2].log_time(Duration::from_secs(13));
        let mut key_phrases = KeyPhrases::new(
            [
                cois[0].id, cois[0].id, cois[0].id, cois[1].id, cois[1].id, cois[1].id, cois[2].id,
                cois[2].id, cois[2].id,
            ],
            [
                KeyPhrase::new("key", arr1(&[3., 1., 1.])).unwrap(),
                KeyPhrase::new("phrase", arr1(&[2., 1., 1.])).unwrap(),
                KeyPhrase::new("words", arr1(&[1., 1., 1.])).unwrap(),
                KeyPhrase::new("and", arr1(&[1., 6., 1.])).unwrap(),
                KeyPhrase::new("more", arr1(&[1., 5., 1.])).unwrap(),
                KeyPhrase::new("stuff", arr1(&[1., 4., 1.])).unwrap(),
                KeyPhrase::new("still", arr1(&[1., 1., 9.])).unwrap(),
                KeyPhrase::new("not", arr1(&[1., 1., 8.])).unwrap(),
                KeyPhrase::new("enough", arr1(&[1., 1., 7.])).unwrap(),
            ],
        );
        let config = Config::default();

        let top_key_phrases_first = key_phrases.take(
            &cois,
            usize::MAX,
            config.horizon(),
            config.penalty(),
            config.gamma(),
        );

        let top_key_phrases_second = key_phrases.take(
            &cois,
            usize::MAX,
            config.horizon(),
            config.penalty(),
            config.gamma(),
        );

        assert_eq!(top_key_phrases_first, top_key_phrases_second);
    }

    mod clean_key_phrase {
        use super::*;

        #[test]
        fn no_symbol_is_identity_letters() {
            let s = "aàáâäąbßcçdeèéêëęfghiìíîïlłmnǹńoòóôöpqrsśtuùúüvwyỳýÿzź";
            assert_eq!(clean_key_phrase(s), s);
        }

        #[test]
        fn no_symbol_is_identity_numbers() {
            let s = "0123456789";
            assert_eq!(clean_key_phrase(s), s);
        }

        #[test]
        fn remove_symbols() {
            assert_eq!(clean_key_phrase("!$\",?(){};:."), "");
        }

        #[test]
        fn remove_symbols_adjust_space_between() {
            for s in ["a-b", "a - b"] {
                assert_eq!(clean_key_phrase(s), "a b");
            }
        }

        #[test]
        fn remove_symbols_adjust_space_after() {
            for s in ["a!  ", "a ! ", "a  !  "] {
                assert_eq!(clean_key_phrase(s), "a");
            }
        }

        #[test]
        fn remove_symbols_adjust_space_before() {
            for s in ["  !a ", " ! a ", "  !  a  "] {
                assert_eq!(clean_key_phrase(s), "a");
            }
        }

        #[test]
        fn adjust_spaces() {
            assert_eq!(clean_key_phrase("  a  b  c  "), "a b c");
        }
    }
}
