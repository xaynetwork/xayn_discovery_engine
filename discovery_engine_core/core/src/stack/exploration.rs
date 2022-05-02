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

use std::{cmp::Reverse, collections::BTreeSet};

use itertools::{chain, Itertools};
use ndarray::{Array2, ArrayView1};
use rand::{prelude::IteratorRandom, Rng};
use xayn_ai::{
    cosine_similarity,
    ranker::{pairwise_cosine_similarity, CoiPoint, NegativeCoi, PositiveCoi},
};

use crate::{document::Document, utils::nan_safe_f32_cmp};

/// Configurations for the exploration stack.
pub(crate) struct Config {
    number_of_candidates: usize,
    /// The maximum number of documents to keep.
    max_selected_docs: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            number_of_candidates: 10,
            max_selected_docs: 20,
        }
    }
}

pub(crate) fn document_selection(
    positive_cois: &[PositiveCoi],
    negative_cois: &[NegativeCoi],
    documents: Vec<Document>,
    config: &Config,
) -> Vec<Document> {
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
) -> Vec<Document>
where
    R: Rng + ?Sized,
{
    let document_embeddings = documents
        .iter()
        .map(|document| document.smbert_embedding.view())
        .collect_vec();

    let pos_cois = positive_cois.iter().map(|coi| coi.point().view());
    let neg_cois = negative_cois.iter().map(|coi| coi.point().view());

    let cois = chain!(pos_cois, neg_cois).collect_vec();
    // #TODO Panic
    let nearest_coi_for_docs = find_nearest_coi_for_docs(&document_embeddings, &cois);
    let doc_similarities = pairwise_cosine_similarity(document_embeddings.into_iter());

    // #TODO Panic
    let selected = select_by_randomization_with_threshold(
        &doc_similarities,
        &nearest_coi_for_docs,
        config.number_of_candidates,
        config.max_selected_docs,
        rng,
    );

    retain_documents_by_indexes(selected, documents)
}

fn select_by_randomization_with_threshold<R>(
    doc_similarities: &Array2<f32>,
    nearest_coi_for_docs: &[f32],
    number_of_candidates: usize,
    max_selected_docs: usize,
    rng: &mut R,
) -> Vec<usize>
where
    R: Rng + ?Sized,
{
    // #TODO Panic
    let (threshold, mut candidates) =
        select_initial_candidates(nearest_coi_for_docs, number_of_candidates);

    let mut selected = Vec::new();
    while !candidates.is_empty() && selected.len() < max_selected_docs {
        let chosen_doc = *candidates
            .iter()
            .choose(rng)
            .unwrap(/* the condition of the `while` ensures that `candidates` can't be empty */);

        // remove all docs that have a similarity to chosen doc >= threshold
        let to_remove = doc_similarities
            .row(chosen_doc)
            .iter()
            .enumerate()
            .filter(|(_, doc_sim)| *doc_sim >= &threshold)
            .map(|(idx, _)| idx)
            .collect();
        candidates = &candidates - &to_remove;

        selected.push(chosen_doc);
    }

    selected
}

/// Returns the indices that would sort an array.
fn argsort(arr: &[f32]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..arr.len()).collect();
    indices.sort_unstable_by(move |&i, &j| nan_safe_f32_cmp(&arr[i], &arr[j]));
    indices
}

/// Computes the cosine similarity between the cois and documents and returns the
/// cosine similarity of the nearest coi for each document.
///
/// # Panics
/// Panics if `cois` is empty.
fn find_nearest_coi_for_docs(
    docs: &[ArrayView1<'_, f32>],
    cois: &[ArrayView1<'_, f32>],
) -> Vec<f32> {
    // creates a cosine similarity matrix (docs x cois)
    // |      | coi1                  | coi2                  |
    // | doc1 | `cos_sim(doc1, coi1)` | `cos_sim(doc1, coi2)` |
    // | doc2 | `cos_sim(doc2, coi1)` | `cos_sim(doc2, coi2)` |
    //
    // finds the nearest coi for each document.
    // [doc1(max(cos_sim1, cos_sim2, ...)), doc2(max(cos_sim1, cos_sim2, ...)), ...]
    docs.iter()
        .cartesian_product(cois)
        .map(|(a, b)| cosine_similarity(*a, *b))
        .chunks(cois.len())
        .into_iter()
        .map(|similarities_for_doc| {
            similarities_for_doc
                .max_by(nan_safe_f32_cmp)
                .unwrap_or_default()
        })
        .collect()
}

/// Determines the threshold and returns it together with the indexes of the documents
/// that are below that threshold.
///
/// # Panics
/// Panics if `number_of_candidates >= nearest_cois_for_docs.len()`
fn select_initial_candidates(
    nearest_coi_for_docs: &[f32],
    number_of_candidates: usize,
) -> (f32, BTreeSet<usize>) {
    let mut idxs = argsort(nearest_coi_for_docs);
    let threshold = idxs
        .get(number_of_candidates)
        .map(|idx| nearest_coi_for_docs.get(*idx).unwrap())
        .unwrap();
    let selectable_idxs = idxs.drain(..number_of_candidates).collect();
    (*threshold, selectable_idxs)
}

fn retain_documents_by_indexes(
    mut selected: Vec<usize>,
    documents: Vec<Document>,
) -> Vec<Document> {
    selected.sort_unstable_by_key(|w| Reverse(*w));
    let mut retain = Vec::with_capacity(selected.len());
    for (idx, doc) in documents.into_iter().enumerate() {
        if selected.is_empty() {
            break;
        } else if idx == *selected.last().unwrap(/* can't be empty because of the check above */) {
            retain.push(doc);
            selected.pop();
        }
    }
    retain
}

#[cfg(test)]
mod tests {
    use std::iter::FromIterator;

    use ndarray::{arr1, ArrayBase, FixedInitializer};
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    use test_utils::{assert_approx_eq, uuid::mock_uuid};
    use xayn_ai::CoiId;

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
    #[should_panic]
    fn test_find_nearest_coi_for_docs_no_cois() {
        find_nearest_coi_for_docs(
            &[arr1(&[1., 1., 0.]).view()],
            &[] as &[ArrayView1<'_, f32>; 0],
        );
    }

    #[test]
    fn test_find_nearest_coi_for_docs_no_docs() {
        let max = find_nearest_coi_for_docs(
            &[] as &[ArrayView1<'_, f32>; 0],
            &[arr1(&[1., 1., 0.]).view()],
        );

        assert!(max.is_empty());
    }

    #[test]
    fn test_find_nearest_coi_for_docs() {
        let cois = vec![
            arr1(&[1., 4., 0.]),
            arr1(&[3., 1., 0.]),
            arr1(&[4., 1., 0.]),
        ];
        let documents = vec![arr1(&[1., 1., 0.]), arr1(&[-1., 1., 0.])];
        let max = find_nearest_coi_for_docs(
            &documents.iter().map(ArrayBase::view).collect_vec(),
            &cois.iter().map(ArrayBase::view).collect_vec(),
        );

        assert_approx_eq!(f32, max, [0.894_427_2, 0.514_495_8]);
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
    fn test_select_initial_candidates() {
        let (threshold, idxs) = select_initial_candidates(&[3., 4., 2., 0.], 3);
        assert_approx_eq!(f32, threshold, 4.);
        assert_eq!(idxs, BTreeSet::from_iter([3, 2, 0, 1]));

        let (threshold_, idxs) = select_initial_candidates(&[3., 4., 2., 0.], 0);
        assert_approx_eq!(f32, threshold_, 0.);
        assert_eq!(idxs, BTreeSet::new());
    }

    #[test]
    fn test_retain_documents_by_indexes() {
        let docs = vec![new_doc(), new_doc(), new_doc(), new_doc(), new_doc()];
        let expected = vec![docs[0].id, docs[2].id, docs[4].id];
        let retained = retain_documents_by_indexes(vec![4, 2, 0], docs);

        assert_eq!(retained[0].id, expected[0]);
        assert_eq!(retained[1].id, expected[1]);
        assert_eq!(retained[2].id, expected[2]);
    }

    #[test]
    fn test_retain_documents_by_indexes_non_selected() {
        let docs = vec![new_doc(), new_doc(), new_doc()];
        let retained = retain_documents_by_indexes(vec![], docs);
        assert!(retained.is_empty());
    }

    #[test]
    fn test_retain_documents_by_indexes_no_docs() {
        let retained = retain_documents_by_indexes(vec![4, 2, 0], vec![]);
        assert!(retained.is_empty());
    }

    #[test]
    fn test_document_selection() {
        // doc0 is close to neg_coi0
        // doc2 is close to pos_coi0
        // doc1 and 3 should be selected
        let docs =
            new_docs_with_embeddings(&[[3., 1., 0.], [-1., 0., 0.], [2., 3., 0.], [-3., 7., 0.]]);

        let expected = vec![docs[1].id, docs[3].id];

        let pos_cois = create_pos_cois(&[[4., 4., 0.], [5., 5., 0.], [6., 4., 0.]]);
        let neg_cois = create_neg_cois(&[[4., 1., 0.], [5., 1., 0.], [5., -1., 0.]]);

        let config = Config {
            number_of_candidates: 2,
            max_selected_docs: 10,
        };
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let docs = document_selection_with_rng(&pos_cois, &neg_cois, docs, &config, &mut rng);

        assert_eq!(docs[0].id, expected[0]);
        assert_eq!(docs[1].id, expected[1]);
    }

    #[test]
    fn test_document_selection_close_document() {
        // doc0 is close to neg_coi0
        // doc2 is close to pos_coi0
        // doc4 is close to doc3 but doc3 is further away from pos_coi0 then doc4
        // doc1 and 3 should be selected although number_of_candidates is 3
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
            max_selected_docs: 10,
        };
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let docs = document_selection_with_rng(&pos_cois, &neg_cois, docs, &config, &mut rng);

        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].id, expected[0]);
        assert_eq!(docs[1].id, expected[1]);
    }
}
