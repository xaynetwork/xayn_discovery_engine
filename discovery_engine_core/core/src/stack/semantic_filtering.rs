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

use displaydoc::Display;
use kodama::{linkage, Dendrogram, Method};
use ndarray::ArrayView1;
use thiserror::Error;
use xayn_ai::ranker::cosine_similarity;

use crate::document::Document;

/// Semantic clustering errors.
#[derive(Error, Debug, Display)]
pub enum Error {
    /// No enough documents.
    NotEnoughDocuments,
}

fn determine_semantic_clusters(
    documents: &[Document],
    method: Method,
    distance_threshold: f32,
) -> Result<Vec<usize>, Error> {
    if documents.len() < 2 {
        return Err(Error::NotEnoughDocuments);
    }

    let length = (documents.len() * (documents.len() - 1)) / 2;
    let mut condensed_distance_matrix = Vec::with_capacity(length);
    for row in 0..documents.len() - 1 {
        for col in row + 1..documents.len() {
            let distance = cosine_distance(
                documents[row].smbert_embedding.view(),
                documents[col].smbert_embedding.view(),
            );
            condensed_distance_matrix.push(distance);
        }
    }

    let dendrogram = linkage(&mut condensed_distance_matrix, documents.len(), method);
    let labels = cut_tree(&dendrogram, distance_threshold);
    Ok(labels)
}

fn cut_tree(dendrogram: &Dendrogram<f32>, distance_threshold: f32) -> Vec<usize> {
    // at the beginning every sample is in its own cluster
    let clusters = (0..dendrogram.observations())
        .map(|x| (x, vec![x]))
        .collect::<BTreeMap<_, _>>();

    // merge clusters until threshold is reached
    let (_, clusters) = dendrogram
        .steps()
        .iter()
        .take_while(|step| step.dissimilarity < distance_threshold)
        .fold(
            (dendrogram.observations(), clusters),
            |(cluster_id, mut clusters), step| {
                let mut cluster1 = clusters.remove(&step.cluster1).unwrap();
                let mut cluster2 = clusters.remove(&step.cluster2).unwrap();

                // merge clusters
                cluster1.append(&mut cluster2);

                clusters.insert(cluster_id, cluster1);
                (cluster_id + 1, clusters)
            },
        );

    // assign labels to samples
    clusters.into_iter().enumerate().fold(
        vec![0; dendrogram.observations()],
        |mut labels, (label, (_, sample_ids))| {
            sample_ids.iter().for_each(|id| labels[*id] = label);
            labels
        },
    )
}

/// Computes the cosine distance of two vectors.
fn cosine_distance(a: ArrayView1<'_, f32>, b: ArrayView1<'_, f32>) -> f32 {
    1.0 - cosine_similarity(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_no_enough_samples() {
        let res = determine_semantic_clusters(&[], Method::Average, 1.);
        assert!(matches!(res.unwrap_err(), Error::NotEnoughDocuments))
    }

    #[test]
    fn test_cluster_one_documents() {
        let res = determine_semantic_clusters(&[Document::default()], Method::Average, 1.);
        assert!(matches!(res.unwrap_err(), Error::NotEnoughDocuments))
    }

    #[test]
    fn test_cluster_multiple_documents() {
        let res = determine_semantic_clusters(
            &[Document::default(), Document::default()],
            Method::Average,
            1.,
        );
        assert_eq!(res.unwrap(), [0, 0])
    }

    #[test]
    fn test_cut_tree_1_cluster() {
        // cut ─────────────────────────
        //         ┌────┴─────┐
        //         │       ┌──┴─┐
        //         |       |    |
        //       ┌─┴──┐    │    │
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 3.6);
        assert_eq!(labels, [0, 0, 0, 0])
    }

    #[test]
    fn test_cut_tree_3_cluster() {
        //         ┌──────────┐
        //         │       ┌──┴─┐
        // cut ────┼───────┼────┼───────
        //       ┌─┴──┐    │    │
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 0.75);
        assert_eq!(labels, [2, 2, 0, 1])
    }

    #[test]
    fn test_cut_tree_4_cluster() {
        //         ┌──────────┐
        //         │       ┌──┴─┐
        //         |       │    │
        // cut ──┌─┼──┐────┼────┼───────
        //       A    B    C    D
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = cut_tree(&dendrogram, 0.5);
        assert_eq!(labels, [0, 1, 2, 3])
    }
}
