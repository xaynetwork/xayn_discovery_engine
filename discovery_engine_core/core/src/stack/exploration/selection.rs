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

use std::collections::{BTreeSet, HashSet};

use displaydoc::Display;
use itertools::{chain, Itertools};
use ndarray::{Array2, ArrayView1};
use rand::{prelude::IteratorRandom, Rng};
use thiserror::Error;
use xayn_ai::{
    cosine_similarity,
    ranker::{pairwise_cosine_similarity, CoiPoint, NegativeCoi, PositiveCoi},
};

use crate::{document::Document, utils::nan_safe_f32_cmp};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// Not enough cois
    NotEnoughCois,
}

/// Configurations for the exploration stack.
#[derive(Debug)]
pub(crate) struct Config {
    /// The number of candidates.
    number_of_candidates: usize,
    /// The maximum number of documents to keep.
    max_selected_docs: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            number_of_candidates: 40,
            max_selected_docs: 20,
        }
    }
}

/// Selects documents by randomization with a threshold.
///
/// <https://xainag.atlassian.net/wiki/spaces/M2D/pages/2376663041/Exploration+Stack#2.-Pick-a-random-article-and-delete-articles-more-similar-to-it-than-a-threshold>
/// outlines how the selection works.
///
/// # Errors
/// Fails if both positive and negative cois are empty.
pub(crate) fn document_selection(
    positive_cois: &[PositiveCoi],
    negative_cois: &[NegativeCoi],
    documents: Vec<Document>,
    config: &Config,
) -> Result<Vec<Document>, Error> {
    document_selection_with_rng(
        positive_cois,
        negative_cois,
        documents,
        config,
        &mut rand::thread_rng(),
    )
}

fn document_selection_with_rng<R>(
    positive_cois: &[PositiveCoi],
    negative_cois: &[NegativeCoi],
    documents: Vec<Document>,
    config: &Config,
    rng: &mut R,
) -> Result<Vec<Document>, Error>
where
    R: Rng + ?Sized,
{
    if positive_cois.is_empty() && negative_cois.is_empty() {
        return Err(Error::NotEnoughCois);
    }

    let document_embeddings = documents
        .iter()
        .map(|document| document.smbert_embedding.view())
        .collect_vec();

    let pos_cois = positive_cois.iter().map(|coi| coi.point().view());
    let neg_cois = negative_cois.iter().map(|coi| coi.point().view());

    let cois = chain!(pos_cois, neg_cois).collect_vec();
    // max_cosine_similarity can't panic because we make sure beforehand
    // that both positive and negative cois aren't empty
    let nearest_coi_for_docs = max_cosine_similarity(&document_embeddings, &cois);
    let doc_similarities = pairwise_cosine_similarity(document_embeddings.into_iter());

    let selected = select_by_randomization_with_threshold(
        &doc_similarities,
        &nearest_coi_for_docs,
        config.number_of_candidates,
        config.max_selected_docs,
        rng,
    );

    Ok(retain_documents_by_indices(&selected, documents))
}

fn select_by_randomization_with_threshold<R>(
    doc_similarities: &Array2<f32>,
    nearest_coi_for_docs: &[f32],
    number_of_candidates: usize,
    max_selected_docs: usize,
    rng: &mut R,
) -> HashSet<usize>
where
    R: Rng + ?Sized,
{
    let (threshold, mut candidates) =
        select_initial_candidates(nearest_coi_for_docs, number_of_candidates);

    let mut selected = HashSet::new();
    while !candidates.is_empty() && selected.len() < max_selected_docs {
        let chosen_doc = *candidates
            .iter()
            .choose(rng)
            .unwrap(/* the condition of the `while` ensures that `candidates` can't be empty */);

        // remove all docs that have a similarity to chosen_doc >= threshold
        let to_remove = doc_similarities
            .row(chosen_doc)
            .indexed_iter()
            .filter_map(|(idx, doc_sim)| (*doc_sim >= threshold).then(|| idx))
            .collect();
        candidates = &candidates - &to_remove;

        selected.insert(chosen_doc);
    }

    selected
}

/// Returns the indices that would sort an array.
fn argsort(arr: &[f32]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..arr.len()).collect();
    indices.sort_unstable_by(|&i, &j| nan_safe_f32_cmp(&arr[i], &arr[j]));
    indices
}

/// Computes the cosine similarity between the cois and documents and returns the
/// cosine similarity of the nearest coi for each document.
///
/// # Panics
/// Panics if `cois` is empty.
fn max_cosine_similarity(docs: &[ArrayView1<'_, f32>], cois: &[ArrayView1<'_, f32>]) -> Vec<f32> {
    // creates a cosine similarity matrix (docs x cois)
    // |      | coi1                  | coi2                  |
    // | doc1 | `cos_sim(doc1, coi1)` | `cos_sim(doc1, coi2)` |
    // | doc2 | `cos_sim(doc2, coi1)` | `cos_sim(doc2, coi2)` |
    //
    // finds the nearest coi for each document
    // [doc1(max(cos_sim1, cos_sim2, ...)), doc2(max(cos_sim1, cos_sim2, ...)), ...]
    docs.iter()
        .map(|&doc| {
            cois.iter()
                .map(|&coi| cosine_similarity(doc, coi))
                .max_by(nan_safe_f32_cmp)
                .unwrap()
        })
        .collect()
}

/// Determines the threshold and returns it together with the indices of the documents
/// that are below that threshold.
///
/// If `number_of_candidates > nearest_coi_for_docs.len()`, `number_of_candidates`
/// will be the value of `nearest_coi_for_docs.len()`.
///
/// If `number_of_candidates` or `nearest_coi_for_docs.len()` equals `0`, the function will
/// return a threshold of `0.` and an empty set of candidates.
fn select_initial_candidates(
    nearest_coi_for_docs: &[f32],
    number_of_candidates: usize,
) -> (f32, BTreeSet<usize>) {
    let number_of_candidates = number_of_candidates.min(nearest_coi_for_docs.len());
    if number_of_candidates == 0 {
        return (0., BTreeSet::new());
    }

    let mut indices = argsort(nearest_coi_for_docs);
    // number_of_candidates - 1 is safe because we check before that number_of_candidates != 0
    let threshold = nearest_coi_for_docs[indices[number_of_candidates - 1]];
    let candidates = indices.drain(..number_of_candidates).collect();
    (threshold, candidates)
}

/// Retains the documents that have been selected.
fn retain_documents_by_indices(
    selected: &HashSet<usize>,
    documents: Vec<Document>,
) -> Vec<Document> {
    documents
        .into_iter()
        .enumerate()
        .filter_map(|(idx, doc)| selected.contains(&idx).then(|| doc))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use ndarray::{arr1, ArrayBase, FixedInitializer};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use xayn_ai::CoiId;
    use xayn_discovery_engine_test_utils::{assert_approx_eq, uuid::mock_uuid};

    use crate::document::Id;

    use super::*;

    fn new_doc() -> Document {
        Document {
            id: Id::new(),
            ..Document::default()
        }
    }

    fn new_docs_with_embeddings(points: &[impl FixedInitializer<Elem = f32>]) -> Vec<Document> {
        points
            .iter()
            .enumerate()
            .map(|(id, point)| Document {
                id: Id::from(mock_uuid(id)),
                smbert_embedding: arr1(point.as_init_slice()).into(),
                ..Document::default()
            })
            .collect()
    }

    fn create_pos_cois(points: &[impl FixedInitializer<Elem = f32>]) -> Vec<PositiveCoi> {
        points
            .iter()
            .enumerate()
            .map(|(id, point)| {
                PositiveCoi::new(CoiId::from(mock_uuid(id)), arr1(point.as_init_slice()))
            })
            .collect()
    }

    fn create_neg_cois(points: &[impl FixedInitializer<Elem = f32>]) -> Vec<NegativeCoi> {
        points
            .iter()
            .enumerate()
            .map(|(id, point)| {
                NegativeCoi::new(CoiId::from(mock_uuid(id)), arr1(point.as_init_slice()))
            })
            .collect()
    }

    #[test]
    fn test_max_cosine_similarity() {
        let cois = vec![
            arr1(&[1., 4., 0.]),
            arr1(&[3., 1., 0.]),
            arr1(&[4., 1., 0.]),
        ];
        let documents = vec![arr1(&[1., 1., 0.]), arr1(&[-1., 1., 0.])];
        let max = max_cosine_similarity(
            &documents.iter().map(ArrayBase::view).collect_vec(),
            &cois.iter().map(ArrayBase::view).collect_vec(),
        );

        assert_approx_eq!(f32, max, [0.894_427_2, 0.514_495_8]);
    }

    #[test]
    #[should_panic]
    fn test_max_cosine_similarity_no_cois() {
        max_cosine_similarity(
            &[arr1(&[1., 1., 0.]).view()],
            &[] as &[ArrayView1<'_, f32>; 0],
        );
    }

    #[test]
    fn test_max_cosine_similarity_no_docs() {
        let max = max_cosine_similarity(
            &[] as &[ArrayView1<'_, f32>; 0],
            &[arr1(&[1., 1., 0.]).view()],
        );

        assert!(max.is_empty());
    }

    #[test]
    fn test_argsort() {
        let idxs = argsort(&[3., 1., 2.]);
        assert_eq!(idxs, [1, 2, 0]);
    }

    #[test]
    fn test_argsort_inf() {
        let idxs = argsort(&[3., f32::INFINITY, 2.]);
        assert_eq!(idxs, [2, 0, 1]);
    }

    #[test]
    fn test_argsort_neg_inf() {
        let idxs = argsort(&[3., f32::NEG_INFINITY, 2.]);
        assert_eq!(idxs, [1, 2, 0]);
    }

    #[test]
    fn test_argsort_neg_nan() {
        let idxs = argsort(&[3., f32::NAN, 2.]);
        assert_eq!(idxs, [1, 2, 0]);
    }

    #[test]
    fn test_argsort_empty() {
        let idxs = argsort(&[]);
        assert!(idxs.is_empty());
    }

    #[test]
    fn test_select_initial_candidates() {
        let (threshold, idxs) = select_initial_candidates(&[3., 4., 2., 0.], 2);
        assert_approx_eq!(f32, threshold, 2.);
        assert_eq!(idxs, BTreeSet::from_iter([3, 2]));
    }

    #[test]
    fn test_select_initial_candidates_all() {
        let (threshold, idxs) = select_initial_candidates(&[3., 4., 2., 0.], 4);
        assert_approx_eq!(f32, threshold, 4.);
        assert_eq!(idxs, BTreeSet::from_iter([3, 2, 0, 1]));
    }

    #[test]
    fn test_select_initial_candidates_none() {
        let (threshold, idxs) = select_initial_candidates(&[3., 4., 2., -1.], 0);
        assert_approx_eq!(f32, threshold, 0.);
        assert_eq!(idxs, BTreeSet::new());
    }

    #[test]
    fn test_select_initial_candidates_too_many() {
        let (threshold, idxs) = select_initial_candidates(&[3., 4., 2., -1.], 5);
        assert_approx_eq!(f32, threshold, 4.);
        assert_eq!(idxs, BTreeSet::from_iter([3, 2, 0, 1]));

        let (threshold_, idxs) = select_initial_candidates(&[], 5);
        assert_approx_eq!(f32, threshold_, 0.);
        assert_eq!(idxs, BTreeSet::new());
    }

    #[test]
    fn test_retain_documents_by_indices() {
        let docs = vec![new_doc(), new_doc(), new_doc(), new_doc(), new_doc()];
        let expected = vec![docs[0].id, docs[2].id, docs[4].id];
        let retained = retain_documents_by_indices(&HashSet::from_iter([4, 2, 0]), docs);

        assert_eq!(retained[0].id, expected[0]);
        assert_eq!(retained[1].id, expected[1]);
        assert_eq!(retained[2].id, expected[2]);
    }

    #[test]
    fn test_retain_documents_by_indices_non_selected() {
        let docs = vec![new_doc(), new_doc(), new_doc()];
        let retained = retain_documents_by_indices(&HashSet::new(), docs);
        assert!(retained.is_empty());
    }

    #[test]
    fn test_retain_documents_by_indices_no_docs() {
        let retained = retain_documents_by_indices(&HashSet::from_iter([4, 2, 0]), vec![]);
        assert!(retained.is_empty());
    }

    #[test]
    fn test_document_selection() {
        // doc0 is close to neg_coi0
        // doc2 is close to pos_coi0
        // doc1 and 3 should be selected
        let docs =
            new_docs_with_embeddings(&[[3., 1., 0.], [-1., -3., 0.], [2., 3., 0.], [-3., 7., 0.]]);

        let expected = vec![docs[1].id, docs[3].id];

        let pos_cois = create_pos_cois(&[[4., 4., 0.], [5., 5., 0.], [6., 4., 0.]]);
        let neg_cois = create_neg_cois(&[[4., 1., 0.], [5., 1., 0.], [5., -1., 0.]]);

        let config = Config {
            number_of_candidates: 2,
            ..Config::default()
        };
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let docs =
            document_selection_with_rng(&pos_cois, &neg_cois, docs, &config, &mut rng).unwrap();

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, expected[0]);
        assert_eq!(docs[1].id, expected[1]);
    }

    #[test]
    fn test_document_selection_close_document() {
        // doc0 is close to neg_coi0
        // doc2 is close to pos_coi0
        // doc3 and doc4 are far away from pos_coi0 but doc3 is further away from pos_coi0 then doc4
        // doc4 is close to doc3 and should be removed
        // doc1 and 3 should be selected
        let docs = new_docs_with_embeddings(&[
            [3., 1., 0.],
            [-1., 0., 0.],
            [2., 3., 0.],
            [-3., 7., 0.],
            [-2., 5., 0.],
        ]);

        let expected = vec![docs[1].id, docs[3].id];

        let pos_cois = create_pos_cois(&[[4., 4., 0.], [5., 5., 0.], [6., 4., 0.]]);
        let neg_cois = create_neg_cois(&[[4., 1., 0.], [5., 1., 0.], [5., -1., 0.]]);

        let config = Config {
            number_of_candidates: 3,
            ..Config::default()
        };
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let docs =
            document_selection_with_rng(&pos_cois, &neg_cois, docs, &config, &mut rng).unwrap();

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, expected[0]);
        assert_eq!(docs[1].id, expected[1]);
    }

    #[test]
    fn test_document_selection_close_no_documents() {
        let pos_cois = create_pos_cois(&[[4., 4., 0.]]);
        let config = Config {
            number_of_candidates: 10,
            ..Config::default()
        };
        let docs = document_selection(&pos_cois, &[], vec![], &config).unwrap();
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_document_selection_close_no_candidates() {
        let docs = new_docs_with_embeddings(&[[3., 1., 0.]]);
        let pos_cois = create_pos_cois(&[[4., 4., 0.]]);
        let config = Config {
            number_of_candidates: 0,
            ..Config::default()
        };
        let docs = document_selection(&pos_cois, &[], docs, &config).unwrap();
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_document_selection_close_all_documents() {
        let docs =
            new_docs_with_embeddings(&[[3., 1., 0.], [-1., 0., 0.], [2., 3., 0.], [-3., 7., 0.]]);
        let pos_cois = create_pos_cois(&[[4., 4., 0.]]);
        let config = Config {
            number_of_candidates: 4,
            ..Config::default()
        };
        let docs = document_selection(&pos_cois, &[], docs, &config).unwrap();
        assert_eq!(docs.len(), 4);
    }

    #[test]
    fn test_document_selection_close_no_cois() {
        let res = document_selection(&[], &[], vec![], &Config::default());
        assert!(matches!(res.unwrap_err(), Error::NotEnoughCois));
    }

    #[test]
    fn test_document_selection_close_more_than_max() {
        let docs = new_docs_with_embeddings(&[
            [3., 3., 1.],
            [-3., -3., 1.],
            [-3., 3., 1.],
            [3., -3., 1.],
            [3., 0., 1.],
            [-3., 0., 1.],
            [0., 3., 1.],
            [0., -3., 1.],
        ]);
        let expected = vec![docs[4].id, docs[5].id];
        let pos_cois = create_pos_cois(&[[0., 0., 1.]]);
        let config = Config {
            number_of_candidates: 8,
            max_selected_docs: 2,
        };
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        // with max_selected_docs > 3, it will select 4, 5, 6, 7
        let docs = document_selection_with_rng(&pos_cois, &[], docs, &config, &mut rng).unwrap();
        assert_eq!(docs.len(), 2);

        assert_eq!(docs[0].id, expected[0]);
        assert_eq!(docs[1].id, expected[1]);
    }
}
