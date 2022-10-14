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
use ndarray::ArrayView1;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use xayn_discovery_engine_ai::{l2_norm, nan_safe_f32_cmp};

use crate::document::{Document, WeightedSource};

use super::source_weight;

/// Computes the cosine similarity of two documents' embeddings.
///
/// # Panics
/// Panics if the indices `i` or `j` are out of bounds.
#[inline]
fn cosine_similarity(documents: &[Document], norms: &[f32], i: usize, j: usize) -> f32 {
    if norms[i] <= 0. || norms[j] <= 0. {
        return 1.0;
    }

    (documents[i]
        .smbert_embedding
        .view()
        .dot(&documents[j].smbert_embedding.view())
        / norms[i]
        / norms[j])
        .clamp(-1., 1.)
}

/// Computes the date distance (in days) of two documents' publication dates.
///
/// # Panics
/// Panics if the indices `i` or `j` are out of bounds.
#[inline]
#[allow(clippy::cast_possible_truncation)] // distance is small enough
fn date_distance(documents: &[Document], i: usize, j: usize) -> usize {
    (documents[i].resource.date_published - documents[j].resource.date_published)
        .num_days()
        .unsigned_abs() as usize
}

/// Computes the decayed date distance.
///
/// # Panics
/// Panics if the index `distance` is out of bounds.
#[inline]
fn decay_factor(distance: usize, max_days: usize, exp_max_days: f32, threshold: f32) -> f32 {
    if max_days <= distance {
        return threshold;
    }

    static EXP_DISTANCES: Lazy<Vec<f32>> = Lazy::new(|| {
        (0..365)
            .into_par_iter()
            .map(
                #[allow(clippy::cast_precision_loss)] // distance is small enough
                |distance| (-0.1 * distance as f32).exp(),
            )
            .collect()
    });

    ((exp_max_days - EXP_DISTANCES[distance]) / (exp_max_days - 1.)).max(0.) * (1. - threshold)
        + threshold
}

/// Computes the condensed combined normalized distance matrix.
#[inline]
fn condensed_normalized_distance(combined: Vec<f32>, min: f32, max: f32) -> Vec<f32> {
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

    assign_labels(clusters, dendrogram.observations())
}

fn find_n_clusters(dendrogram: &Dendrogram<f32>, n_clusters: usize) -> Vec<usize> {
    // at the beginning every sample is in its own cluster
    // we use BTreeMap instead of HashMap to keep the order of the labels with the
    // order of the documents
    let mut clusters = (0..dendrogram.observations())
        .map(|x| (x, vec![x]))
        .collect::<BTreeMap<_, _>>();

    let mut id = dendrogram.observations();
    for step in dendrogram.steps() {
        if clusters.len() <= n_clusters {
            break;
        }
        // unwrap safety:
        // - initial cluster ids have been inserted in the beginning
        // - merged cluster ids have been inserted in a previous iteration/step
        let mut cluster_1 = clusters.remove(&step.cluster1).unwrap();
        let cluster_2 = clusters.remove(&step.cluster2).unwrap();

        // merge clusters
        cluster_1.extend(cluster_2);
        clusters.insert(id, cluster_1);

        id += 1;
    }

    assign_labels(clusters, dendrogram.observations())
}

/// Assigns the cluster ids to labels beginning from `0`.
fn assign_labels(clusters: BTreeMap<usize, Vec<usize>>, len: usize) -> Vec<usize> {
    clusters
        .into_values()
        .enumerate()
        .fold(vec![0; len], |mut labels, (label, cluster)| {
            for sample in cluster {
                labels[sample] = label;
            }
            labels
        })
}

fn compute_norms(documents: &[Document]) -> Vec<f32> {
    documents
        .into_par_iter()
        .map(|document| l2_norm(document.smbert_embedding.view()))
        .collect()
}

fn triangular_indices(size: usize) -> Vec<(usize, usize)> {
    (0..size)
        .flat_map(|row| (row + 1..size).map(move |column| (row, column)))
        .collect()
}

/// Calculates the normalized distances.
fn normalized_distance(documents: &[Document], config: &SemanticFilterConfig) -> Vec<f32> {
    let norms = compute_norms(documents);
    #[allow(clippy::cast_precision_loss)] // max_days is usually small enough
    let exp_max_days = (-0.1 * config.max_days as f32).exp();
    // simplified to a single loop, where the indiviudal values are calculated and finally returned as factor
    let combined = triangular_indices(documents.len())
        .into_par_iter()
        .map(|(i, j)| {
            let distance = date_distance(documents, i, j);
            let decay = decay_factor(distance, config.max_days, exp_max_days, config.threshold);

            if decay == 0. {
                0.
            } else {
                cosine_similarity(documents, &norms, i, j) * decay
            }
        })
        .collect::<Vec<_>>();

    let (min, max) = combined
        .iter()
        .copied()
        .minmax_by(nan_safe_f32_cmp)
        .into_option()
        .unwrap_or_default();

    condensed_normalized_distance(combined, min, max)
}

/// Configurations for semantic filtering.
pub(crate) struct SemanticFilterConfig {
    /// Maximum days threshold after which documents fully decay.
    pub(crate) max_days: usize,
    /// Threshold to scale the time decay factor.
    pub(crate) threshold: f32,
    /// The criterion when to stop merging the clusters.
    pub(crate) criterion: Criterion,
}

/// The criterion when to stop merging the clusters.
pub(crate) enum Criterion {
    /// Cluster cutoff threshold for dissimilarity of normalized combined distances (must be in the
    /// unit interval [0, 1]).
    MaxDissimilarity(f32),
    /// The max number of cluster.
    MaxClusters(usize),
}

impl Default for SemanticFilterConfig {
    fn default() -> Self {
        Self {
            max_days: 10,
            threshold: 0.5,
            criterion: Criterion::MaxDissimilarity(0.5),
        }
    }
}

/// Filters the documents semantically.
pub(crate) fn filter_semantically(
    documents: Vec<Document>,
    sources: &[WeightedSource],
    config: &SemanticFilterConfig,
) -> Vec<Document> {
    if documents.len() < 2 {
        return documents;
    }

    let mut normalized_distance = normalized_distance(&documents, config);
    let dendrogram = linkage(&mut normalized_distance, documents.len(), Method::Average);

    let labels = match config.criterion {
        Criterion::MaxDissimilarity(max_dissimilarity) => cut_tree(&dendrogram, max_dissimilarity),
        Criterion::MaxClusters(max_clusters) => find_n_clusters(&dendrogram, max_clusters),
    };

    // among documents with the same label, keep the one with heaviest source weight
    izip!(labels, documents)
        .into_grouping_map()
        .max_by_key(|_label, doc| source_weight(doc, sources))
        .into_values()
        .collect()
}

/// Computes the cosine similarity between the cois and documents and returns the
/// cosine similarity of the nearest coi for each document.
pub(crate) fn max_cosine_similarity<'a, 'b, I, J>(docs: I, cois: J) -> Vec<f32>
where
    I: IntoIterator<Item = ArrayView1<'a, f32>>,
    J: IntoIterator<Item = ArrayView1<'b, f32>>,
    <J as IntoIterator>::IntoIter: Clone,
{
    let cois = cois.into_iter();
    if cois.clone().next().is_none() {
        return Vec::new();
    }

    // creates a cosine similarity matrix (docs x cois)
    // |      | coi1                  | coi2                  |
    // | doc1 | `cos_sim(doc1, coi1)` | `cos_sim(doc1, coi2)` |
    // | doc2 | `cos_sim(doc2, coi1)` | `cos_sim(doc2, coi2)` |
    //
    // finds the nearest coi for each document
    // [doc1(max(cos_sim1, cos_sim2, ...)), doc2(max(cos_sim1, cos_sim2, ...)), ...]
    docs.into_iter()
        .map(|doc| {
            cois.clone()
                .map(|coi| xayn_discovery_engine_ai::cosine_similarity(doc, coi))
                .max_by(nan_safe_f32_cmp)
                .unwrap(/* cois is not empty */)
        })
        .collect()
}

/// Filters all documents which are too similar to their closest coi.
pub(crate) fn filter_too_similar<'a, I>(
    mut documents: Vec<Document>,
    cois: I,
    threshold: f32,
) -> Vec<Document>
where
    I: IntoIterator<Item = ArrayView1<'a, f32>>,
    <I as IntoIterator>::IntoIter: Clone,
{
    let embeddings = documents
        .iter()
        .map(|document| document.smbert_embedding.view());
    let mut retain = max_cosine_similarity(embeddings, cois)
        .into_iter()
        .map(|similarity| similarity <= threshold);
    documents.retain(|_| retain.next().unwrap_or(true));

    documents
}

#[cfg(test)]
#[allow(clippy::non_ascii_literal)]
mod tests {
    use std::iter::repeat_with;

    use chrono::{TimeZone, Utc};
    use ndarray::aview1;
    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
    use xayn_discovery_engine_test_utils::{assert_approx_eq, smbert};

    use crate::document::NewsResource;

    use super::*;

    #[test]
    fn test_condensed_cosine_similarity() {
        for n in 0..5 {
            let documents = repeat_with(Document::default).take(n).collect_vec();
            let norms = compute_norms(&documents);
            let condensed = triangular_indices(documents.len())
                .into_iter()
                .map(|(i, j)| cosine_similarity(&documents, &norms, i, j))
                .collect_vec();
            if n < 2 {
                assert!(condensed.is_empty());
            } else {
                assert_eq!(condensed.len(), n * (n - 1) / 2);
            }
            assert!(condensed.iter().all(|c| (-1. ..=1.).contains(c)));
        }
    }

    #[test]
    fn test_condensed_date_distance() {
        for n in 0..5 {
            let documents = repeat_with(Document::default).take(n).collect_vec();
            let condensed = triangular_indices(documents.len())
                .into_iter()
                .map(|(i, j)| date_distance(&documents, i, j))
                .collect_vec();
            if n < 2 {
                assert!(condensed.is_empty());
            } else {
                assert_eq!(condensed.len(), n * (n - 1) / 2);
            }
        }
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // max_days is small
    #[allow(clippy::float_cmp)] // exact equality due to maximum function
    fn test_condensed_decay_factor() {
        let max_days = 2;
        let exp_max_days = (-0.1 * max_days as f32).exp();
        let threshold = 0.5;
        for distance in 0..5 {
            for distance in 0..distance {
                let factor = decay_factor(distance, max_days, exp_max_days, threshold);
                if distance < max_days {
                    assert!(factor > threshold);
                } else {
                    assert_eq!(factor, threshold);
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
    fn test_find_1_cluster() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 1);
        assert_eq!(labels, [0, 0, 0, 0]);
    }

    #[test]
    fn test_find_2_clusters() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 2);
        assert_eq!(labels, [0, 0, 1, 1]);
    }

    #[test]
    fn test_find_3_clusters() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 3);
        assert_eq!(labels, [2, 2, 0, 1]);
    }

    #[test]
    fn test_find_4_clusters() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 4);
        assert_eq!(labels, [0, 1, 2, 3]);
    }

    #[test]
    fn test_find_n_clusters_too_many() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 5);
        assert_eq!(labels, [0, 1, 2, 3]);
    }

    #[test]
    fn test_find_0_clusters_no_panic() {
        let dendrogram = linkage(&mut [0.5, 3., 2., 3.5, 2.5, 1.], 4, Method::Single);
        let labels = find_n_clusters(&dendrogram, 0);
        assert_eq!(labels, [0, 0, 0, 0]);
    }

    #[test]
    fn test_filter_semantically_empty() {
        let documents = vec![];
        let sources = &[];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents, sources, &config);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_semantically_single() {
        let documents = vec![Document::default()];
        let sources = &[];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents.clone(), sources, &config);
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
        let sources = &[];
        let config = SemanticFilterConfig::default();
        let filtered = filter_semantically(documents.clone(), sources, &config);
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
        let sources = &[];
        let config = SemanticFilterConfig {
            criterion: Criterion::MaxDissimilarity(0.),
            ..SemanticFilterConfig::default()
        };
        let filtered = filter_semantically(documents.clone(), sources, &config);
        assert_eq!(filtered.len(), 3);
        assert_eq!(filtered[0].id, documents[0].id);
        assert_eq!(filtered[1].id, documents[1].id);
        assert_eq!(filtered[2].id, documents[2].id);
    }

    #[test]
    fn test_normalized_distance() {
        fn new_doc(smbert_embedding: Embedding, secs: i64) -> Document {
            Document {
                smbert_embedding,
                resource: NewsResource {
                    date_published: Utc.timestamp(secs, 0),
                    ..NewsResource::default()
                },
                ..Document::default()
            }
        }

        fn normalized_distance_for_titles(
            titles: &[(&str, i64)],
            smbert: &SMBert,
            expected: &[f32],
        ) {
            let documents = titles
                .iter()
                .map(|(title, secs)| new_doc(smbert.run(title).unwrap(), *secs))
                .collect_vec();
            let distances = normalized_distance(&documents, &SemanticFilterConfig::default());
            assert_approx_eq!(f32, distances, expected);
        }

        let smbert = SMBertConfig::from_files(smbert::vocab().unwrap(), smbert::model().unwrap())
            .unwrap()
            .with_token_size(52)
            .unwrap()
            .with_cleanse_accents(true)
            .with_lower_case(true)
            .with_pooling::<AveragePooler>()
            .build()
            .unwrap();

        let titles_en = [
            ("How To Start A New Life With Less Than $100", 0),
            ("2 Top Reasons to Buy Electric Vehicle", 864_000),
            ("Summer Expected to Be \\u2018Brutally Hot'", 0),
            ("Summer Expected to Be Hot", 0),
        ];

        let expected_en = [1., 0.928_844_15, 0.983_816, 0.828_074_4, 0.823_989_33, 0.];

        normalized_distance_for_titles(&titles_en, &smbert, &expected_en);

        let titles_de = [
            ("Autounfall auf der A10", 0),
            ("Polizei nimmt Tatverdächtigen fest", 864_000),
            ("Das neue Elektroauto", 0),
            ("Wertvoller Hammer gestohlen", 0),
        ];
        let expected_de = [
            0.657_387_55,
            0.,
            0.235_730_89, /* on my windows machine, AMD processor, I get 0.235_731_12 here instead, other values are the same */
            1.,
            0.812_637_87,
            0.559_570_13,
        ];

        normalized_distance_for_titles(&titles_de, &smbert, &expected_de);
    }

    #[test]
    fn test_max_cosine_similarity_no_documents() {
        assert!(max_cosine_similarity([], [aview1(&[1., 1., 0.])]).is_empty());
    }

    #[test]
    fn test_max_cosine_similarity_no_cois() {
        assert!(max_cosine_similarity([aview1(&[1., 1., 0.])], []).is_empty());
    }

    #[test]
    fn test_max_cosine_similarity() {
        let documents = [aview1(&[1., 1., 0.]), aview1(&[-1., 1., 0.])];
        let cois = [
            aview1(&[1., 4., 0.]),
            aview1(&[3., 1., 0.]),
            aview1(&[4., 1., 0.]),
        ];
        let max = max_cosine_similarity(documents, cois);

        assert_approx_eq!(f32, max, [0.894_427_2, 0.514_495_8]);
    }
}
