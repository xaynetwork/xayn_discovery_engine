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

use std::collections::{BTreeMap, HashMap};

use itertools::{izip, Itertools};
use kodama::{linkage, Dendrogram, Method};
use ndarray::ArrayView1;
use xayn_discovery_engine_ai::{cosine_similarity, l2_norm, nan_safe_f32_cmp, triangular_product};

use crate::document::{Document, Id, WeightedSource};

use super::source_weight;

/// Computes the condensed cosine similarity matrix of the documents' embeddings.
#[cfg(test)]
fn condensed_cosine_similarity(documents: &[Document]) -> Vec<f32> {
    let mut norms = HashMap::new();
    triangular_product(documents, |doc_a: &Document, doc_b: &Document| {
        condensed_cosine_similarity_single(doc_a, doc_b, &mut norms)
    })
    .collect()
}

fn condensed_cosine_similarity_single(
    doc_a: &Document,
    doc_b: &Document,
    norms: &mut HashMap<Id, f32>,
) -> f32 {
    let v_a = doc_a.smbert_embedding.view();
    let v_b = doc_b.smbert_embedding.view();
    let ni = *norms.entry(doc_a.id).or_insert_with(|| l2_norm(v_a));
    let nj = *norms.entry(doc_b.id).or_insert_with(|| l2_norm(v_b));

    if ni > 0. && nj > 0. {
        return (v_a.dot(&v_b) / ni / nj).clamp(-1., 1.);
    }

    1.0
}

/// Computes the condensed date distance matrix (in days) of the documents' publication dates.
#[cfg(test)]
fn condensed_date_distance(documents: &[Document]) -> Vec<f32> {
    triangular_product(documents, condensed_date_distance_single).collect()
}

#[allow(clippy::cast_precision_loss)] // day difference is small
fn condensed_date_distance_single(doc_a: &Document, doc_b: &Document) -> f32 {
    (doc_a.resource.date_published - doc_b.resource.date_published)
        .num_days()
        .abs() as f32
}

/// Computes the condensed decayed date distance matrix.
#[cfg(test)]
fn condensed_decay_factor(date_distance: Vec<f32>, max_days: f32, threshold: f32) -> Vec<f32> {
    let exp_max_days = (-0.1 * max_days).exp();
    date_distance
        .into_iter()
        .map(|distance| {
            ((exp_max_days - (-0.1 * distance).exp()) / (exp_max_days - 1.)).max(0.)
                * (1. - threshold)
                + threshold
        })
        .collect()
}

fn condensed_decay_factor_single(distance: f32, exp_max_days: f32, threshold: f32) -> f32 {
    ((exp_max_days - (-0.1 * distance).exp()) / (exp_max_days - 1.)).max(0.) * (1. - threshold)
        + threshold
}

/// Computes the condensed combined normalized distance matrix.
fn condensed_normalized_distance(combined: Vec<f32>) -> Vec<f32> {
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

/// Calculates the normalized distances.
fn normalized_distance(documents: &[Document], config: &SemanticFilterConfig) -> Vec<f32> {
    let mut norms = HashMap::new();
    // simplified to a single loop, where the indiviudal values are calculated and finally returned as factor
    let combined: Vec<f32> = triangular_product(documents, |doc_a: &Document, doc_b: &Document| {
        let similarity = condensed_cosine_similarity_single(doc_a, doc_b, &mut norms);
        let distance = condensed_date_distance_single(doc_a, doc_b);
        let decay = condensed_decay_factor_single(
            distance,
            (-0.1 * config.max_days).exp(),
            config.threshold,
        );

        similarity * decay
    })
    .collect();

    condensed_normalized_distance(combined)
}

/// Configurations for semantic filtering.
pub(crate) struct SemanticFilterConfig {
    /// Maximum days threshold after which documents fully decay (must be non-negative).
    pub(crate) max_days: f32,
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
            max_days: 10.,
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
                .map(|coi| cosine_similarity(doc, coi))
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

    use chrono::NaiveDateTime;
    use ndarray::aview1;
    use xayn_discovery_engine_ai::Embedding;
    use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};
    use xayn_discovery_engine_test_utils::{assert_approx_eq, smbert};
    use xayn_discovery_engine_tokenizer::{AccentChars, CaseChars};

    use crate::document::NewsResource;

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
            let threshold = 0.5;
            let condensed = condensed_decay_factor(date_distance, max_days as f32, threshold);
            for (m, c) in condensed.into_iter().enumerate() {
                if m < max_days {
                    assert!(c > threshold);
                } else {
                    assert_eq!(c, threshold);
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
                    date_published: NaiveDateTime::from_timestamp(secs, 0),
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
                .collect::<Vec<_>>();
            let distances = normalized_distance(&documents, &SemanticFilterConfig::default());
            assert_approx_eq!(f32, distances, expected);
        }

        let smbert = SMBertConfig::from_files(smbert::vocab().unwrap(), smbert::model().unwrap())
            .unwrap()
            .with_token_size(52)
            .unwrap()
            .with_accents(AccentChars::Cleanse)
            .with_case(CaseChars::Lower)
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
            0.235_730_89,
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
