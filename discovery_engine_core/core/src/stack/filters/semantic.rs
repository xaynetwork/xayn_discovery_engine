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

/// Agglomerates clusters wrt the documents' embeddings.
fn determine_semantic_clusters(documents: &[Document], distance_threshold: f32) -> Vec<usize> {
    if documents.len() < 2 {
        return vec![0; documents.len()];
    }

    let mut condensed_dissimilarity_matrix = condensed_cosine_distance(documents);
    debug_assert_eq!(
        condensed_dissimilarity_matrix.len(),
        (documents.len() * (documents.len() - 1)) / 2,
    );

    let dendrogram = linkage(
        &mut condensed_dissimilarity_matrix,
        documents.len(),
        Method::Average,
    );
    cut_tree(&dendrogram, distance_threshold)
}

/// Computes the condensed cosine distance matrix of the documents' embeddings.
fn condensed_cosine_distance(documents: &[Document]) -> Vec<f32> {
    pairwise_cosine_similarity(
        documents
            .iter()
            .map(|document| document.smbert_embedding.view()),
    )
    .indexed_iter()
    .filter_map(|((i, j), &similarity)| (i < j).then(|| 1. - similarity))
    .collect()
}

/// Agglomerates clusters wrt the documents' publication date differences.
fn determine_date_clusters(
    documents: &[Document],
    labels: &[usize],
    date_threshold: f32,
) -> Vec<usize> {
    debug_assert_eq!(documents.len(), labels.len());
    let clusters = labels.iter().enumerate().fold(
        BTreeMap::<_, Vec<_>>::new(),
        |mut clusters, (idx, &label)| {
            clusters.entry(label).or_default().push(idx);
            clusters
        },
    );

    let (_, labels) = clusters.into_values().fold(
        (0, vec![0; labels.len()]),
        |(label, mut labels), cluster| {
            let label =
                determine_date_subcluster(documents, &mut labels, date_threshold, cluster, label);
            (label, labels)
        },
    );

    labels
}

/// Agglomerates subclusters of a semantic cluster wrt the documents' publication date differences.
fn determine_date_subcluster(
    documents: &[Document],
    labels: &mut [usize],
    date_threshold: f32,
    cluster: Vec<usize>,
    label: usize,
) -> usize {
    debug_assert!(cluster.iter().copied().max().unwrap() < labels.len());
    if cluster.len() < 2 {
        labels[cluster[0]] = label;
        return label + 1;
    }

    let mut condensed_dissimilarity_matrix = condensed_date_distance(documents, &cluster);
    debug_assert_eq!(
        condensed_dissimilarity_matrix.len(),
        (cluster.len() * (cluster.len() - 1)) / 2,
    );

    let dendrogram = linkage(
        &mut condensed_dissimilarity_matrix,
        cluster.len(),
        Method::Average,
    );
    let sublabels = cut_tree(&dendrogram, date_threshold);
    let offset = izip!(cluster, sublabels).fold(0, |offset, (idx, sublabel)| {
        labels[idx] = label + sublabel;
        offset.max(sublabel)
    });

    label + 1 + offset
}

/// Computes the condensed date distance matrix (in days) of the documents' publication dates.
fn condensed_date_distance(documents: &[Document], _cluster: &[usize]) -> Vec<f32> {
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

/// Computes the normalized condensed dissimilarity matrix of the documents' embeddings and dates.
#[allow(dead_code)]
fn condensed_distance(documents: &[Document], max_days: f32, threshold: f32) -> Vec<f32> {
    let cosine_distance = condensed_cosine_distance(documents);
    let date_distance = condensed_date_distance(documents, &[]);

    let scale = 1. / ((-0.1 * max_days).exp() - 1.);
    let addend = scale * (-0.1 * max_days).exp();
    let decay_factor = date_distance
        .into_iter()
        .map(|distance| {
            (scale * (threshold - 1.) * ((-0.1 * distance).exp() + addend)).max(0.) + threshold
        })
        .collect::<Vec<_>>();

    let dissimilarity = izip!(cosine_distance, decay_factor)
        .map(|(distance, factor)| distance * factor)
        .collect::<Vec<_>>();
    debug_assert!(dissimilarity
        .iter()
        .all(|&dissimilarity| dissimilarity >= 0.));
    let (min, max) = dissimilarity
        .iter()
        .copied()
        .minmax_by(nan_safe_f32_cmp)
        .into_option()
        .unwrap_or_default();
    let diff = max - min;

    if diff > 0. {
        dissimilarity
            .into_iter()
            .map(|dis| (dis - min) / diff)
            .collect()
    } else {
        // the denominator is zero iff either:
        // - there are less than two documents
        // - all documents have the same embedding and date
        // - all documents decayed because they are too old
        // in either case all documents are treated as similar, ie with zero dissimilarity
        vec![0.; dissimilarity.len()]
    }
}

/// Cuts off the dendrogram at the first step which exceeds the distance/dissimilarity threshold.
fn cut_tree(dendrogram: &Dendrogram<f32>, threshold: f32) -> Vec<usize> {
    // at the beginning every sample is in its own cluster
    let clusters = (0..dendrogram.observations())
        .map(|x| (x, vec![x]))
        .collect::<BTreeMap<_, _>>();

    // merge clusters until threshold is reached
    let (_, clusters) = dendrogram
        .steps()
        .iter()
        .take_while(|step| step.dissimilarity < threshold)
        .fold(
            (dendrogram.observations(), clusters),
            |(id, mut clusters), step| {
                // unwrap safety:
                // - inital cluster ids have been inserted in the beginning
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
    /// Cluster cutoff threshold for dissimilarity of cosine distances.
    distance_threshold: f32,
    /// Cluster cutoff threshold for dissimilarity of date distances.
    date_threshold: f32,
}

impl Default for SemanticFilterConfig {
    fn default() -> Self {
        Self {
            distance_threshold: 0.67,
            date_threshold: 10.,
        }
    }
}

/// Filters the documents semantically.
pub(crate) fn filter_semantically(
    documents: Vec<Document>,
    config: &SemanticFilterConfig,
) -> Vec<Document> {
    let labels = determine_semantic_clusters(&documents, config.distance_threshold);
    let labels = determine_date_clusters(&documents, &labels, config.date_threshold);

    izip!(documents, labels)
        .unique_by(|(_, label)| *label)
        .map(|(document, _)| document)
        .collect()
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_cluster_empty_documents() {
        let labels = determine_semantic_clusters(&[], 1.);
        assert!(labels.is_empty());
    }

    #[test]
    fn test_semantic_cluster_single_document() {
        let labels = determine_semantic_clusters(&[Document::default()], 1.);
        assert_eq!(labels, [0]);
    }

    #[test]
    fn test_semantic_cluster_multiple_documents() {
        let labels = determine_semantic_clusters(&[Document::default(), Document::default()], 1.);
        assert_eq!(labels, [0, 0]);
    }

    #[test]
    fn test_date_cluster_empty_documents() {
        let labels = determine_date_clusters(&[], &[], 1.);
        assert!(labels.is_empty());
    }

    #[test]
    fn test_date_cluster_single_document() {
        let labels = determine_date_clusters(&[Document::default()], &[0], 1.);
        assert_eq!(labels, [0]);
    }

    #[test]
    fn test_date_cluster_multiple_documents() {
        let labels =
            determine_date_clusters(&[Document::default(), Document::default()], &[0, 0], 1.);
        assert_eq!(labels, [0, 0]);
    }

    #[test]
    fn test_date_subcluster() {
        let documents = [
            Document::default(), // 1
            Document::default(), // 0
            Document::default(), // 1
            Document::default(), // 2
        ];
        let label = 0;
        let mut labels = [0, 0, 0, 0];

        let label = determine_date_subcluster(&documents, &mut labels, 1., vec![1], label);
        assert_eq!(label, 1);
        assert_eq!(labels, [0, 0, 0, 0]);

        let label = determine_date_subcluster(&documents, &mut labels, 1., vec![0, 2], label);
        assert_eq!(label, 2);
        assert_eq!(labels, [1, 0, 1, 0]);

        let label = determine_date_subcluster(&documents, &mut labels, 1., vec![3], label);
        assert_eq!(label, 3);
        assert_eq!(labels, [1, 0, 1, 2]);
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
    fn test_filter_semantically_same() {
        let documents = vec![
            Document::default(),
            Document::default(),
            Document::default(),
        ];
        let config = SemanticFilterConfig {
            distance_threshold: 1.,
            date_threshold: 1.,
        };
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
            distance_threshold: 0.,
            date_threshold: 0.,
        };
        let filtered = filter_semantically(documents.clone(), &config);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].id, documents[0].id);
        assert_eq!(filtered[1].id, documents[1].id);
        assert_eq!(filtered[2].id, documents[2].id);
    }
}
