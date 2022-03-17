// Copyright 2022 Xayn AG
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

use std::collections::BTreeMap;

use itertools::{izip, Itertools};
use kodama::{linkage, Dendrogram, Method};
use xayn_ai::ranker::pairwise_cosine_similarity;

use crate::{document::Document, utils::nan_safe_f32_cmp};

/// Computes the condensed cosine similarity matrix of the documents' embeddings.
fn condensed_cosine_similarity(documents: &[Document]) -> Vec<f32> {
    pairwise_cosine_similarity(
        documents
            .iter()
            .map(|document| document.smbert_embedding.view()),
    )
    .indexed_iter()
    .filter_map(|((i, j), &similarity)| (i < j).then(|| similarity))
    .collect()
}

/// Computes the condensed date distance matrix (in days) of the documents' publication dates.
fn condensed_date_distance(documents: &[Document]) -> Vec<f32> {
    let dates = || {
        documents
            .iter()
            .map(|document| document.resource.date_published)
            .enumerate()
    };

    dates()
        .cartesian_product(dates())
        .filter_map(|((i, this), (j, other))| {
            #[allow(clippy::cast_precision_loss)] // day difference is small
            (i < j).then(|| (this - other).num_days().abs() as f32)
        })
        .collect()
}

/// Computes the condensed decayed date distance matrix.
fn condensed_decay_factor(
    date_distance: Vec<f32>,
    max_days: f32,
    max_dissimilarity: f32,
) -> Vec<f32> {
    let exp_max_days = (-0.1 * max_days).exp();
    date_distance
        .into_iter()
        .map(|distance| {
            ((exp_max_days - (-0.1 * distance).exp()) / (exp_max_days - 1.)).max(0.)
                * (1. - max_dissimilarity)
                + max_dissimilarity
        })
        .collect()
}

/// Computes the condensed combined normalized distance matrix.
fn condensed_normalized_distance(cosine_similarity: Vec<f32>, decay_factor: Vec<f32>) -> Vec<f32> {
    let combined = izip!(cosine_similarity, decay_factor)
        .map(|(similarity, factor)| similarity * factor)
        .collect::<Vec<_>>();
    let (min, max) = combined
        .iter()
        .copied()
        .minmax_by(nan_safe_f32_cmp)
        .into_option()
        .unwrap_or_default();
    let diff = max - min;

    if diff > 0. {
        combined
            .into_iter()
            .map(|dis| 1. - ((dis - min) / diff))
            .collect()
    } else {
        // the denominator is zero iff either:
        // - there are less than two documents
        // - all documents have the same embedding and date
        // - all documents decayed because they are too old
        // in either case all documents are treated as similar, ie with minimum distance
        vec![0.; combined.len()]
    }
}

/// Cuts off the dendrogram at the first step which exceeds the distance/dissimilarity threshold.
fn cut_tree(dendrogram: &Dendrogram<f32>, max_dissimilarity: f32) -> Vec<usize> {
    // at the beginning every sample is in its own cluster
    let clusters = (0..dendrogram.observations())
        .map(|x| (x, vec![x]))
        .collect::<BTreeMap<_, _>>();

    // merge clusters until threshold is reached
    let (_, clusters) = dendrogram
        .steps()
        .iter()
        .take_while(|step| step.dissimilarity < max_dissimilarity)
        .fold(
            (dendrogram.observations(), clusters),
            |(id, mut clusters), step| {
                // unwrap safety:
                // - initial cluster ids have been inserted in the beginning
                // - merged cluster ids have been inserted in a previous iteration/step
                let mut cluster_1 = clusters.remove(&step.cluster1).unwrap();
                let cluster_2 = clusters.remove(&step.cluster2).unwrap();

                // merge clusters
                cluster_1.extend(cluster_2);
                clusters.insert(id, cluster_1);

                (id + 1, clusters)
            },
        );

    // assign labels to samples
    clusters.into_values().enumerate().fold(
        vec![0; dendrogram.observations()],
        |mut labels, (label, cluster)| {
            for sample in cluster {
                labels[sample] = label;
            }
            labels
        },
    )
}

/// Configurations for semantic filtering.
pub(crate) struct SemanticFilterConfig {
    /// Maximum days threshold after which documents fully decay (must be non-negative).
    max_days: f32,
    /// Cluster cutoff threshold for dissimilarity of normalized combined distances (must be in the
    /// unit interval [0, 1]).
    max_dissimilarity: f32,
}

impl Default for SemanticFilterConfig {
    fn default() -> Self {
        Self {
            max_days: 10.,
            max_dissimilarity: 0.67,
        }
    }
}

/// Filters the documents semantically.
pub(crate) fn filter_semantically(
    documents: Vec<Document>,
    config: &SemanticFilterConfig,
) -> Vec<Document> {
    if documents.len() < 2 {
        return documents;
    }

    let cosine_similarity = condensed_cosine_similarity(&documents);
    let date_distance = condensed_date_distance(&documents);
    let decay_factor =
        condensed_decay_factor(date_distance, config.max_days, config.max_dissimilarity);
    let mut normalized_distance = condensed_normalized_distance(cosine_similarity, decay_factor);

    let dendrogram = linkage(&mut normalized_distance, documents.len(), Method::Average);
    let labels = cut_tree(&dendrogram, config.max_dissimilarity);

    izip!(documents, labels)
        .unique_by(|(_, label)| *label)
        .map(|(document, _)| document)
        .collect()
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod tests {
    use std::iter::repeat_with;

    use super::*;

    #[test]
    fn test_condensed_cosine_similarity() {
        for n in 0..5 {
            let documents = repeat_with(Document::default).take(n).collect::<Vec<_>>();
            let condensed = condensed_cosine_similarity(&documents);
            if n < 2 {
                assert!(condensed.is_empty());
            } else {
                assert_eq!(condensed.len(), n * (n - 1) / 2);
            }
            assert!(condensed.iter().all(|c| (-1. ..=1.).contains(c)));
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // c represents whole days
    fn test_condensed_date_distance() {
        for n in 0..5 {
            let documents = repeat_with(Document::default).take(n).collect::<Vec<_>>();
            let condensed = condensed_date_distance(&documents);
            if n < 2 {
                assert!(condensed.is_empty());
            } else {
                assert_eq!(condensed.len(), n * (n - 1) / 2);
            }
            assert!(condensed.into_iter().all(|c| 0. <= c && c == c.trunc()));
        }
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // d is small
    #[allow(clippy::float_cmp)] // exact equality due to maximum function
    fn test_condensed_decay_factor() {
        for n in 0..5 {
            let date_distance = (0..n).map(|d| d as f32).collect();
            let max_days = 2;
            let max_dissimilarity = 0.5;
            let condensed =
                condensed_decay_factor(date_distance, max_days as f32, max_dissimilarity);
            for (m, c) in condensed.into_iter().enumerate() {
                if m < max_days {
                    assert!(c > max_dissimilarity);
                } else {
                    assert_eq!(c, max_dissimilarity);
                }
            }
        }
    }

    #[test]
    fn test_cut_tree_1_cluster() {
        // cut ─────────┼───────────────
        //         ┌────┴─────┐
        //         │       ┌──┴─┐
        //       ┌─┴──┐    │    │
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 3.6);
        assert_eq!(labels, [0, 0, 0, 0]);
    }

    #[test]
    fn test_cut_tree_3_clusters() {
        //         ┌────┴─────┐
        //         │       ┌──┴─┐
        // cut ────┼───────┼────┼───────
        //       ┌─┴──┐    │    │
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 0.75);
        assert_eq!(labels, [2, 2, 0, 1]);
    }

    #[test]
    fn test_cut_tree_4_clusters() {
        //         ┌────┴─────┐
        //         │       ┌──┴─┐
        //       ┌─┴──┐    │    │
        // cut ──┼────┼────┼────┼───────
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 0.5);
        assert_eq!(labels, [0, 1, 2, 3]);
    }

    #[test]
    fn test_filter_semantically_empty() {
        let documents = vec![];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents, &config);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_semantically_single() {
        let documents = vec![Document::default()];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents.clone(), &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, documents[0].id);
    }

    #[test]
    fn test_filter_semantically_same() {
        let documents = vec![
            Document::default(),
            Document::default(),
            Document::default(),
        ];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents.clone(), &config);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, documents[0].id);
    }

    #[test]
    fn test_filter_semantically_different() {
        let documents = vec![
            Document::default(),
            Document::default(),
            Document::default(),
        ];
        let config = SemanticFilterConfig {
            max_dissimilarity: 0.,
            ..SemanticFilterConfig::default()
        };
        let filtered = filter_semantically(documents.clone(), &config);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].id, documents[0].id);
        assert_eq!(filtered[1].id, documents[1].id);
        assert_eq!(filtered[2].id, documents[2].id);
    }
}
