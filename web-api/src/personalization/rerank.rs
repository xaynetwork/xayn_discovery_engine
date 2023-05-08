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

use std::{cmp::Ordering, collections::HashMap, hash::BuildHasher};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{nan_safe_f32_cmp, nan_safe_f32_cmp_desc, CoiSystem, UserInterests};

use crate::{
    models::{DocumentId, DocumentProperties, DocumentTag, PersonalizedDocument},
    personalization::PersonalizationConfig,
};

fn rank_keys_by_score<K, S>(
    keys_with_score: impl IntoIterator<Item = (K, S)>,
    mut sort_by: impl FnMut(&S, &S) -> Ordering,
) -> impl Iterator<Item = (K, f32)> {
    keys_with_score
        .into_iter()
        .sorted_unstable_by(|(_, s1), (_, s2)| sort_by(s1, s2))
        .enumerate()
        .map(
            #[allow(clippy::cast_precision_loss)] // index is small enough
            |(index, (key, _))| (key, 1. / (1 + index) as f32),
        )
}

fn rerank_by_interest(
    coi_system: &CoiSystem,
    documents: &[PersonalizedDocument],
    interests: &UserInterests,
    time: DateTime<Utc>,
) -> HashMap<DocumentId, f32> {
    let scores = coi_system.score(documents, interests, time);
    rank_keys_by_score(
        documents
            .iter()
            .map(|document| document.id.clone())
            .zip(scores),
        nan_safe_f32_cmp_desc,
    )
    .collect()
}

fn rerank_by_tag_weight(
    documents: &[PersonalizedDocument],
    tag_weights: &HashMap<DocumentTag, usize>,
) -> HashMap<DocumentId, f32> {
    let mut weights = HashMap::<_, Vec<_>>::with_capacity(documents.len());
    for document in documents {
        let weight = document
            .tags
            .iter()
            .map(|tag| tag_weights.get(tag).copied().unwrap_or_default())
            .sum::<usize>();
        weights.entry(weight).or_default().push(document.id.clone());
    }

    rank_keys_by_score(
        weights
            .into_iter()
            .map(|(weight, documents)| (documents, weight)),
        |w1, w2| w1.cmp(w2).reverse(),
    )
    .flat_map(|(documents, score)| documents.into_iter().map(move |document| (document, score)))
    .collect()
}

/// Reranks documents based on a combination of their interest, tag weight and elasticsearch scores.
///
/// The `score_weights` determine the ratios of the scores, it is ordered as
/// `[interest_weight, tag_weight, elasticsearch_weight]`. The final score/ranking per document is
/// calculated as the weighted sum of the scores.
pub(super) fn rerank_by_scores(
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

#[doc(hidden)]
pub fn bench_rerank<S>(
    coi_system: &CoiSystem,
    documents: Vec<(NormalizedEmbedding, Vec<String>)>,
    interests: &UserInterests,
    tag_weights: HashMap<String, usize, S>,
    time: DateTime<Utc>,
) where
    S: BuildHasher,
{
    // small allocation overhead, but we don't have to expose a lot of private items
    let mut documents = documents
        .into_iter()
        .enumerate()
        .map(|(id, (embedding, tags))| PersonalizedDocument {
            id: id.to_string().try_into().unwrap(),
            score: 1.0,
            embedding,
            properties: DocumentProperties::default(),
            tags: tags
                .into_iter()
                .map(|tag| tag.try_into().unwrap())
                .collect_vec(),
        })
        .collect_vec();
    let tag_weights = tag_weights
        .into_iter()
        .map(|(tag, weight)| (tag.try_into().unwrap(), weight))
        .collect();
    let score_weights = PersonalizationConfig::default().score_weights;
    rerank_by_scores(
        coi_system,
        &mut documents,
        interests,
        &tag_weights,
        score_weights,
        time,
    );
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, time::Duration};

    use xayn_ai_bert::Embedding1;
    use xayn_ai_coi::{CoiConfig, CoiId, CoiStats, PositiveCoi};
    use xayn_test_utils::assert_approx_eq;

    use super::*;

    fn mock_documents(n: usize) -> Vec<PersonalizedDocument> {
        (0..n)
            .map(|i| {
                let id = i.to_string().try_into().unwrap();

                let mut embedding = vec![1.0; n];
                embedding[i] = 10.0;
                let embedding = Embedding1::from(embedding).normalize().unwrap();

                let tags = if i % 2 == 0 {
                    vec!["general".try_into().unwrap()]
                } else {
                    vec![
                        "general".try_into().unwrap(),
                        "specific".try_into().unwrap(),
                    ]
                };

                PersonalizedDocument {
                    id,
                    score: 1.0,
                    embedding,
                    properties: DocumentProperties::default(),
                    tags,
                }
            })
            .collect()
    }

    fn mock_coi(i: usize, n: usize, time: DateTime<Utc>) -> PositiveCoi {
        let id = CoiId::new();

        let mut point = vec![0.0; n];
        point[i] = 1.0;
        let point = Embedding1::from(point).normalize().unwrap();

        let stats = CoiStats {
            view_count: i + 1,
            view_time: Duration::ZERO,
            last_view: time,
        };

        PositiveCoi { id, point, stats }
    }

    fn sort_ids(scores: HashMap<DocumentId, f32>) -> Vec<String> {
        scores
            .into_iter()
            .sorted_by(|(_, s1), (_, s2)| nan_safe_f32_cmp_desc(s1, s2))
            .map(|(document, _)| document.to_string())
            .collect()
    }

    #[test]
    fn test_rank_keys_by_score_empty() {
        assert!(rank_keys_by_score::<(), ()>([], |_, _| unreachable!())
            .next()
            .is_none());
    }

    #[test]
    fn test_rank_keys_by_similar_scores() {
        let scores = [(0, 0), (1, 0), (2, 0), (3, 1), (4, 2), (5, 2)];
        let keys = rank_keys_by_score(scores, |s1, s2| s1.cmp(s2).reverse())
            .map(|(key, _)| key)
            .collect_vec();
        assert_eq!(
            keys[0..=1].iter().copied().collect::<HashSet<_>>(),
            [4, 5].into(),
        );
        assert!(keys[0] == 4 || keys[0] == 5);
        assert!(keys[1] == 4 || keys[1] == 5);
        assert_eq!(keys[2], 3);
        assert!(keys[3] == 0 || keys[3] == 1 || keys[3] == 2);
        assert!(keys[4] == 0 || keys[4] == 1 || keys[4] == 2);
        assert!(keys[5] == 0 || keys[5] == 1 || keys[5] == 2);
    }

    #[test]
    fn test_rank_key_by_different_scores() {
        let scores = (0..=6).enumerate().map(|(score, key)| (key, score));
        let keys = rank_keys_by_score(scores, |s1, s2| s1.cmp(s2).reverse())
            .map(|(key, _)| key)
            .collect_vec();
        assert_eq!(keys, (0..=6).rev().collect_vec());
    }

    #[test]
    fn test_rerank_by_interest_empty() {
        let coi_system = CoiConfig::default().build();
        let documents = Vec::default();
        let interests = UserInterests::default();
        let time = Utc::now();

        assert!(rerank_by_interest(&coi_system, &documents, &interests, time).is_empty());
    }

    #[test]
    fn test_rerank_without_interests() {
        let coi_system = CoiConfig::default().build();
        let documents = mock_documents(5);
        let interests = UserInterests::default();
        let time = Utc::now();

        let reranked = rerank_by_interest(&coi_system, &documents, &interests, time);
        assert_eq!(sort_ids(reranked), ["0", "1", "2", "3", "4"]);
    }

    #[test]
    fn test_rerank_with_interest() {
        let n = 5;
        let coi_system = CoiConfig::default().build();
        let documents = mock_documents(n);
        let time = Utc::now();
        let interests = UserInterests {
            positive: vec![mock_coi(1, n, time), mock_coi(4, n, time)],
            negative: vec![],
        };

        let reranked = rerank_by_interest(&coi_system, &documents, &interests, time);
        assert_eq!(sort_ids(reranked), ["4", "1", "0", "2", "3"]);
    }

    #[test]
    fn test_rerank_by_tag_weight_empty() {
        let documents = Vec::default();
        let tag_weights = HashMap::default();

        assert!(rerank_by_tag_weight(&documents, &tag_weights).is_empty());
    }

    #[test]
    fn test_rerank_without_tag_weights() {
        let n = 5;
        let documents = mock_documents(n);
        let tag_weights = HashMap::default();

        let reranked = rerank_by_tag_weight(&documents, &tag_weights);
        for i in 1..n {
            assert_approx_eq!(
                f32,
                reranked[&"0".try_into().unwrap()],
                reranked[&i.to_string().try_into().unwrap()],
            );
        }
    }

    #[test]
    fn test_rerank_with_tag_weights() {
        let n = 5;
        let documents = mock_documents(n);
        let tag_weights = [
            ("general".try_into().unwrap(), 4),
            ("specific".try_into().unwrap(), 1),
            ("other".try_into().unwrap(), 3),
        ]
        .into();

        let reranked = rerank_by_tag_weight(&documents, &tag_weights);
        assert!(reranked[&"0".try_into().unwrap()] < reranked[&"1".try_into().unwrap()]);
        for i in (2..n).step_by(2) {
            assert_approx_eq!(
                f32,
                reranked[&"0".try_into().unwrap()],
                reranked[&i.to_string().try_into().unwrap()],
            );
        }
        for i in (3..n).step_by(2) {
            assert_approx_eq!(
                f32,
                reranked[&"1".try_into().unwrap()],
                reranked[&i.to_string().try_into().unwrap()],
            );
        }
    }
}
