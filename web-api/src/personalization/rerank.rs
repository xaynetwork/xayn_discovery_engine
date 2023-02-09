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
use xayn_ai_coi::{nan_safe_f32_cmp_desc, CoiSystem, UserInterests};

use crate::models::{DocumentId, DocumentTag, PersonalizedDocument};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(super) struct Rank(usize);

impl Rank {
    pub(super) fn from_index(index: usize) -> Rank {
        Rank(index + 1)
    }

    pub(super) fn to_score(self) -> f32 {
        #![allow(clippy::cast_precision_loss)]
        1. / self.0 as f32
    }

    pub(super) fn merge_as_score(self, other: Rank, ratio: f32) -> f32 {
        self.to_score() * ratio + other.to_score() * (1. - ratio)
    }
}

pub(super) fn rerank_by_interest(
    coi_system: &CoiSystem,
    documents: &[PersonalizedDocument],
    interest: &UserInterests,
    time: DateTime<Utc>,
) -> HashMap<DocumentId, Rank> {
    let scores = coi_system.score(documents, interest, time);
    pairs_to_rank_map(
        izip!(documents.iter().map(|doc| doc.id.clone()), scores),
        nan_safe_f32_cmp_desc,
    )
}

pub(super) fn rerank_by_tag_weights(
    documents: &[PersonalizedDocument],
    tag_weights: &HashMap<DocumentTag, usize>,
) -> HashMap<DocumentId, Rank> {
    let weighted_documents = documents.iter().map(|doc| {
        let weight = doc
            .tags
            .iter()
            .map(|tag| tag_weights.get(tag).copied().unwrap_or_default())
            .sum::<usize>();

        (doc.id.clone(), weight)
    });

    pairs_to_rank_map(weighted_documents, |w1, w2| w1.cmp(w2).reverse())
}

pub(super) fn rerank_by_interest_and_tag_weight(
    coi_system: &CoiSystem,
    documents: &mut [PersonalizedDocument],
    interests: &UserInterests,
    tag_weights: &HashMap<DocumentTag, usize>,
    interest_tag_bias: f32,
    time: DateTime<Utc>,
) {
    let interest_ranks = rerank_by_interest(coi_system, documents, interests, time);
    let tag_weight_ranks = rerank_by_tag_weights(documents, tag_weights);

    for document in documents.iter_mut() {
        document.score = interest_ranks[&document.id]
            .merge_as_score(tag_weight_ranks[&document.id], interest_tag_bias);
    }

    let secondary_sorting_factor = if interest_tag_bias >= 0.5 {
        interest_ranks
    } else {
        tag_weight_ranks
    };

    documents.sort_unstable_by(|a, b| {
        nan_safe_f32_cmp_desc(&a.score, &b.score).then_with(|| {
            secondary_sorting_factor
                .get(&a.id)
                .cmp(&secondary_sorting_factor.get(&b.id))
        })
    });
}

fn pairs_to_rank_map<K, S>(
    keys_with_score: impl IntoIterator<Item = (K, S)>,
    mut sort_by: impl FnMut(&S, &S) -> Ordering,
) -> HashMap<K, Rank>
where
    K: Eq + Hash,
{
    keys_with_score
        .into_iter()
        .sorted_unstable_by(|(_, s1), (_, s2)| sort_by(s1, s2))
        .enumerate()
        .map(|(index, (key, _))| (key, Rank::from_index(index)))
        .collect()
}
