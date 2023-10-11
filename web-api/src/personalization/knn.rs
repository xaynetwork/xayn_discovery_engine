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

use std::{collections::HashMap, time::Duration};

use chrono::{DateTime, Utc};
use futures_util::{stream::FuturesUnordered, Stream, StreamExt};
use itertools::Itertools;
use tracing::error;
use xayn_ai_coi::{compute_coi_weights, Coi};

use crate::{
    error::common::InternalError,
    models::{PersonalizedDocument, SnippetId},
    personalization::filter::Filter,
    rank_merge::{rrf_score, DEFAULT_RRF_K},
    storage::{self, Exclusions, KnnSearchParams, SearchStrategy},
    Error,
};

/// KNN search based on Centers of Interest.
pub(super) struct CoiSearch<'a, I> {
    pub(super) interests: I,
    pub(super) excluded: &'a Exclusions,
    pub(super) horizon: Duration,
    pub(super) max_cois: usize,
    pub(super) count: usize,
    pub(super) num_candidates: usize,
    pub(super) time: DateTime<Utc>,
    pub(super) include_properties: bool,
    pub(super) include_snippet: bool,
    pub(super) filter: Option<&'a Filter>,
}

impl<'a, I> CoiSearch<'a, I>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Clone + Iterator<Item = &'a Coi>,
{
    /// Performs an approximate knn search for documents similar to the user interests.
    pub(super) async fn run_on(
        self,
        storage: &impl storage::Document,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let interests = self.interests.into_iter();
        let coi_weights = compute_coi_weights(interests.clone(), self.horizon, self.time);
        let cois = interests
            .zip(coi_weights)
            .sorted_by(|(coi1, w1), (coi2, w2)| {
                w1.total_cmp(w2)
                    .then_with(|| coi1.id.cmp(&coi2.id))
                    .reverse()
            })
            .take(self.max_cois)
            .collect_vec();

        let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

        let document_futures = cois
            .iter()
            .map(|(coi, weight)| async {
                // weights_sum can't be zero, because all coi_weights are in [0, 1] and at least one of them is > 0
                let weight = *weight / weights_sum;
                let count = weighted_number(weight, self.count);
                let num_candidates = weighted_number(weight, self.num_candidates).max(count);
                storage::Document::get_by_embedding(
                    storage,
                    KnnSearchParams {
                        excluded: self.excluded,
                        embedding: &coi.point,
                        count,
                        num_candidates,
                        strategy: SearchStrategy::Knn,
                        include_properties: self.include_properties,
                        include_snippet: self.include_snippet,
                        filter: self.filter,
                        with_raw_scores: false,
                    },
                )
                .await
            })
            .collect::<FuturesUnordered<_>>();

        let (all_documents, errors) = merge_knn_searchs(document_futures).await;

        if all_documents.is_empty() && !errors.is_empty() {
            Err(InternalError::from_message("Fetching documents failed").into())
        } else {
            Ok(all_documents.into_values().collect_vec())
        }
    }
}

#[allow(
    // fine if number is small enough
    clippy::cast_precision_loss,
    // fine if weight is between 0 and 1
    clippy::cast_sign_loss,
    // weight * number is small enough if the previous assumptions are fine
    clippy::cast_possible_truncation
)]
fn weighted_number(weight: f32, number: usize) -> usize {
    (weight * number as f32).ceil() as usize
}

async fn merge_knn_searchs(
    mut results: impl Stream<Item = Result<Vec<PersonalizedDocument>, Error>> + Unpin,
) -> (HashMap<SnippetId, PersonalizedDocument>, Vec<Error>) {
    let mut all_documents = HashMap::new();
    let mut errors = Vec::new();
    while let Some(result) = results.next().await {
        match result {
            Ok(documents) => {
                // the same document can be returned with different elastic scores, hence the
                // documents are deduplicated and only the highest score is retained for each
                for (idx, mut document) in documents.into_iter().enumerate() {
                    document.score = rrf_score(DEFAULT_RRF_K, idx, 1.0);
                    all_documents
                        .entry(document.id.clone())
                        .and_modify(|PersonalizedDocument { score, .. }| {
                            *score += document.score;
                        })
                        .or_insert(document);
                }
            }
            Err(error) => {
                error!("Error fetching documents: {error}");
                errors.push(error);
            }
        };
    }
    (all_documents, errors)
}

#[cfg(test)]
mod tests {
    use xayn_ai_coi::CoiConfig;

    use super::*;
    use crate::{personalization::PersonalizationConfig, storage::memory::Storage};

    #[tokio::test]
    async fn test_search_knn_documents_for_empty_cois() {
        let storage = Storage::default();
        let documents = CoiSearch {
            interests: &[],
            excluded: &Exclusions::default(),
            horizon: CoiConfig::default().horizon(),
            max_cois: PersonalizationConfig::default().max_cois_for_knn,
            count: 10,
            num_candidates: 10,
            time: Utc::now(),
            include_properties: false,
            include_snippet: false,
            filter: None,
        }
        .run_on(&storage)
        .await
        .unwrap();
        assert!(documents.is_empty());
    }
}
