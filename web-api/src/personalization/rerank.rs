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

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{Coi, CoiSystem};

use crate::{
    models::{
        DocumentEmbedding,
        DocumentId,
        DocumentProperties,
        DocumentTag,
        PersonalizedDocument,
    },
    personalization::PersonalizationConfig,
};

fn rerank_by_interest<'a>(
    coi_system: &CoiSystem,
    documents: &'a [PersonalizedDocument],
    interests: &[Coi],
    time: DateTime<Utc>,
) -> HashMap<&'a DocumentId, f32> {
    coi_system
        .score(documents, interests, time)
        .map(|scores| {
            documents
                .iter()
                .map(|document| &document.id)
                .zip(scores)
                .collect()
        })
        .unwrap_or_default()
}

fn rerank_by_tag_weight<'a>(
    documents: &'a [PersonalizedDocument],
    tag_weights: &HashMap<DocumentTag, usize>,
) -> HashMap<&'a DocumentId, f32> {
    let total_tag_weight = tag_weights.values().sum::<usize>();
    if total_tag_weight == 0 {
        return HashMap::new();
    }
    #[allow(clippy::cast_precision_loss)]
    let total_tag_weight = total_tag_weight as f32;

    documents
        .iter()
        .map(|document| {
            #[allow(clippy::cast_precision_loss)]
            let weight = document
                .tags
                .iter()
                .map(|tag| tag_weights.get(tag).copied().unwrap_or_default())
                .sum::<usize>() as f32;
            (&document.id, weight / total_tag_weight)
        })
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
    interests: &[Coi],
    tag_weights: &HashMap<DocumentTag, usize>,
    score_weights: [f32; 3],
    time: DateTime<Utc>,
) {
    let interest_scores = rerank_by_interest(coi_system, documents, interests, time);
    let tag_weight_scores = rerank_by_tag_weight(documents, tag_weights);

    let scores = documents
        .iter()
        .map(|document| {
            let interest_score = interest_scores
                .get(&document.id)
                .copied()
                .unwrap_or_default();
            let tag_weight_score = tag_weight_scores
                .get(&document.id)
                .copied()
                .unwrap_or_default();
            score_weights[0] * interest_score
                + score_weights[1] * tag_weight_score
                + score_weights[2] * document.score
        })
        .collect_vec();
    for (document, score) in documents.iter_mut().zip(scores) {
        document.score = score;
    }

    documents.sort_unstable_by(|d1, d2| d1.score.total_cmp(&d2.score).reverse());
}

#[doc(hidden)]
pub fn bench_rerank<S>(
    coi_system: &CoiSystem,
    documents: Vec<(NormalizedEmbedding, Vec<String>)>,
    interests: &[Coi],
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
            score: 1.,
            embeddings: vec![DocumentEmbedding::whole_document(embedding)],
            splits: HashSet::new(),
            properties: DocumentProperties::default(),
            tags: tags
                .into_iter()
                .map(|tag| tag.try_into().unwrap())
                .collect_vec()
                .try_into()
                .unwrap(),
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
    use std::time::Duration;

    use xayn_ai_bert::Embedding1;
    use xayn_ai_coi::{Coi, CoiConfig, CoiId, CoiStats};
    use xayn_test_utils::assert_approx_eq;

    use super::*;

    fn mock_documents(n: usize) -> Vec<PersonalizedDocument> {
        (0..n)
            .map(|i| {
                let id = i.to_string().try_into().unwrap();

                let mut embedding = vec![1.; n];
                embedding[i] = 10.;
                let embedding = Embedding1::from(embedding).normalize().unwrap();

                let tags = if i % 2 == 0 {
                    vec!["general".try_into().unwrap()]
                } else {
                    vec![
                        "general".try_into().unwrap(),
                        "specific".try_into().unwrap(),
                    ]
                }
                .try_into()
                .unwrap();

                PersonalizedDocument {
                    id,
                    score: 1.,
                    embeddings: vec![DocumentEmbedding::whole_document(embedding)],
                    splits: HashSet::new(),
                    properties: DocumentProperties::default(),
                    tags,
                }
            })
            .collect()
    }

    fn mock_coi(i: usize, n: usize, time: DateTime<Utc>) -> Coi {
        let id = CoiId::new();

        let mut point = vec![0.; n];
        point[i] = 1.;
        let point = Embedding1::from(point).normalize().unwrap();

        let stats = CoiStats {
            view_count: i + 1,
            view_time: Duration::ZERO,
            last_view: time,
        };

        Coi { id, point, stats }
    }

    #[test]
    fn test_rerank_by_interest_empty() {
        let coi_system = CoiConfig::default().build();
        let documents = Vec::default();
        let interests = Vec::default();
        let time = Utc::now();

        assert!(rerank_by_interest(&coi_system, &documents, &interests, time).is_empty());
    }

    #[test]
    fn test_rerank_without_interests() {
        let coi_system = CoiConfig::default().build();
        let documents = mock_documents(5);
        let interests = Vec::default();
        let time = Utc::now();

        assert!(rerank_by_interest(&coi_system, &documents, &interests, time).is_empty());
    }

    #[test]
    fn test_rerank_with_interest() {
        let n = 5;
        let coi_system = CoiConfig::default().build();
        let documents = mock_documents(n);
        let time = Utc::now();
        let interests = vec![mock_coi(1, n, time), mock_coi(4, n, time)];

        let reranked = rerank_by_interest(&coi_system, &documents, &interests, time);
        let zero = "0".try_into().unwrap();
        let one = "1".try_into().unwrap();
        let two = "2".try_into().unwrap();
        let three = "3".try_into().unwrap();
        let four = "4".try_into().unwrap();
        assert!(0. <= reranked[&&zero]);
        assert_approx_eq!(f32, reranked[&&zero], reranked[&&two]);
        assert_approx_eq!(f32, reranked[&&zero], reranked[&&three]);
        assert!(reranked[&&zero] < reranked[&&one]);
        assert!(reranked[&&one] < reranked[&&four]);
        assert!(reranked[&&four] <= 1.);
    }

    #[test]
    fn test_rerank_by_tag_weight_empty() {
        let documents = Vec::default();
        let tag_weights = [
            ("general".try_into().unwrap(), 4),
            ("specific".try_into().unwrap(), 1),
            ("other".try_into().unwrap(), 3),
        ]
        .into();

        assert!(rerank_by_tag_weight(&documents, &tag_weights).is_empty());
    }

    #[test]
    fn test_rerank_without_tag_weights() {
        let n = 5;
        let documents = mock_documents(n);
        let tag_weights = HashMap::default();

        assert!(rerank_by_tag_weight(&documents, &tag_weights).is_empty());
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
        let zero = "0".try_into().unwrap();
        let one = "1".try_into().unwrap();
        assert!(reranked[&&zero] < reranked[&&one]);
        for i in (2..n).step_by(2) {
            let id = i.to_string().try_into().unwrap();
            assert_approx_eq!(f32, reranked[&&zero], reranked[&&id]);
        }
        for i in (3..n).step_by(2) {
            let id = i.to_string().try_into().unwrap();
            assert_approx_eq!(f32, reranked[&&one], reranked[&&id]);
        }
    }
}
