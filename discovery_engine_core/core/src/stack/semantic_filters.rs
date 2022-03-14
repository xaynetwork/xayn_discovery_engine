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

use kodama::{linkage, Dendrogram, Method};
use xayn_ai::ranker::pairwise_cosine_similarity;

use crate::document::Document;

/// Determines the clusters with an agglomerative clustering approach.
#[allow(dead_code)]
fn determine_semantic_clusters(
    documents: &[Document],
    method: Method,
    distance_threshold: f32,
) -> Vec<usize> {
    if documents.len() < 2 {
        return vec![0; documents.len()];
    }

    let mut condensed_dissimilarity_matrix = condensed_cosine_distance(documents);
    debug_assert_eq!(
        condensed_dissimilarity_matrix.len(),
        (documents.len() * (documents.len() - 1)) / 2,
    );

    let dendrogram = linkage(&mut condensed_dissimilarity_matrix, documents.len(), method);
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
                let mut cluster1 = clusters.remove(&step.cluster1).unwrap();
                let cluster2 = clusters.remove(&step.cluster2).unwrap();

                // merge clusters
                cluster1.extend(cluster2);
                clusters.insert(id, cluster1);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_empty_documents() {
        let labels = determine_semantic_clusters(&[], Method::Average, 1.);
        assert!(labels.is_empty());
    }

    #[test]
    fn test_cluster_single_document() {
        let labels = determine_semantic_clusters(&[Document::default()], Method::Average, 1.);
        assert_eq!(labels, [0]);
    }

    #[test]
    fn test_cluster_multiple_documents() {
        let labels = determine_semantic_clusters(
            &[Document::default(), Document::default()],
            Method::Average,
            1.,
        );
        assert_eq!(labels, [0, 0]);
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
        assert_eq!(labels, [0, 0, 0, 0])
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
        assert_eq!(labels, [2, 2, 0, 1])
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
        assert_eq!(labels, [0, 1, 2, 3])
    }
}
