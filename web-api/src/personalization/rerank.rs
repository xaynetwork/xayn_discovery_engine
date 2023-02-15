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

use std::{cmp::Ordering, collections::HashMap, hash::Hash};

use chrono::{DateTime, Utc};
use itertools::{izip, Itertools};
use xayn_ai_coi::{nan_safe_f32_cmp, nan_safe_f32_cmp_desc, CoiSystem, UserInterests};

use crate::models::{DocumentId, DocumentTag, PersonalizedDocument};

pub(super) fn rerank_by_interest(
    coi_system: &CoiSystem,
    documents: &[PersonalizedDocument],
    interests: &UserInterests,
    time: DateTime<Utc>,
) -> HashMap<DocumentId, f32> {
    let scores = coi_system.score(documents, interests, time);
    rank_keys_by_score(
        izip!(documents.iter().map(|doc| doc.id.clone()), scores),
        nan_safe_f32_cmp_desc,
    )
}

pub(super) fn rerank_by_tag_weight(
    documents: &[PersonalizedDocument],
    tag_weights: &HashMap<DocumentTag, usize>,
) -> HashMap<DocumentId, f32> {
    let weighted_documents = documents.iter().map(|doc| {
        let weight = doc
            .tags
            .iter()
            .map(|tag| tag_weights.get(tag).copied().unwrap_or_default())
            .sum::<usize>();

        (doc.id.clone(), weight)
    });

    rank_keys_by_score(weighted_documents, |w1, w2| w1.cmp(w2).reverse())
}

/// Reranks documents based on a combination of their interest, tag weight and elasticsearch scores.
///
/// The `score_weights` determine the ratios of the scores, it is ordered as
/// `[interest_weight, tag_weight, elasticsearch_weight]`. The final score/ranking per document is
/// calculated as the weighted sum of the scores.
pub(super) fn rerank_by_score_interest_and_tag_weight(
    coi_system: &CoiSystem,
    documents: &mut [PersonalizedDocument],
    interests: &UserInterests,
    tag_weights: &HashMap<DocumentTag, usize>,
    score_weights: [f32; 3],
    time: DateTime<Utc>,
) {
    let interest_scores = rerank_by_interest(coi_system, documents, interests, time);
    let tag_weight_scores = rerank_by_tag_weight(documents, tag_weights);
    let mut elasticsearch_scores = HashMap::with_capacity(documents.len());

    for document in documents.iter_mut() {
        elasticsearch_scores.insert(document.id.clone(), document.score);
        document.score = score_weights[0] * interest_scores[&document.id]
            + score_weights[1] * tag_weight_scores[&document.id]
            + score_weights[2] * document.score;
    }

    let max_score_weight = score_weights.into_iter().max_by(nan_safe_f32_cmp);
    let secondary_sorting_factor = match score_weights
        .into_iter()
        .position(|score_weight| Some(score_weight) >= max_score_weight)
    {
        Some(0) => interest_scores,
        Some(1) => tag_weight_scores,
        Some(2) => elasticsearch_scores,
        _ => unreachable!(),
    };

    documents.sort_unstable_by(|a, b| {
        nan_safe_f32_cmp_desc(&a.score, &b.score).then_with(|| {
            nan_safe_f32_cmp_desc(
                &secondary_sorting_factor[&a.id],
                &secondary_sorting_factor[&b.id],
            )
        })
    });
}

fn rank_keys_by_score<K, S>(
    keys_with_score: impl IntoIterator<Item = (K, S)>,
    mut sort_by: impl FnMut(&S, &S) -> Ordering,
) -> HashMap<K, f32>
where
    K: Eq + Hash,
{
    keys_with_score
        .into_iter()
        .sorted_unstable_by(|(_, s1), (_, s2)| sort_by(s1, s2))
        .enumerate()
        .map(
            #[allow(clippy::cast_precision_loss)] // index is small enough
            |(index, (key, _))| (key, 1. / (1 + index) as f32),
        )
        .collect()
}
