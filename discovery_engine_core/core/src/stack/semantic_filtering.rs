use std::collections::BTreeMap;

use kodama::{linkage, Dendrogram, Method};
use xayn_ai::ranker::cosine_distance;

use crate::document::Document;

fn determine_semantic_clusters(
    documents: &[Document],
    method: Method,
    distance_threshold: f32,
) -> Vec<usize> {
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
    cut_tree(&dendrogram, distance_threshold)
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
