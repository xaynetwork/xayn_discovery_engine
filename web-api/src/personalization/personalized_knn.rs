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
use xayn_ai_coi::{
    compute_coi_relevances,
    compute_coi_weights,
    nan_safe_f32_cmp,
    system_time_now,
    PositiveCoi,
};

use crate::{
    error::common::InternalError,
    models::{DocumentId, PersonalizedDocument},
    storage::{self, KnnSearchParams},
    Error,
};

pub(super) struct Search<'a> {
    pub(super) interests: &'a [PositiveCoi],
    pub(super) excluded: &'a [DocumentId],
    pub(super) horizon: Duration,
    pub(super) max_cois: usize,
    pub(super) count: usize,
    pub(super) published_after: Option<DateTime<Utc>>,
}

impl Search<'_> {
    /// Performs an approximate knn search for documents similar to the positive user interests.
    pub(super) async fn run_on(
        self,
        storage: &impl storage::Document,
    ) -> Result<Vec<PersonalizedDocument>, Error> {
        let Search {
            horizon,
            max_cois,
            count,
            published_after,
            interests,
            excluded,
        } = self;

        let coi_weights = compute_coi_weights(interests, horizon);
        let cois = interests
            .iter()
            .zip(coi_weights)
            .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
            .take(max_cois)
            .collect_vec();

        let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

        let document_futures = cois
            .iter()
            .map(|(coi, weight)| async {
                // weights_sum can't be zero, because coi weights will always return some weights that are > 0
                let weight = *weight / weights_sum;
                #[allow(
                // fine as max documents count is small enough
                clippy::cast_precision_loss,
                // fine as weight should be between 0 and 1
                clippy::cast_sign_loss,
                // fine as number of neighbors is small enough
                clippy::cast_possible_truncation
            )]
                let k_neighbors = (weight * count as f32).ceil() as usize;

                storage::Document::get_by_embedding(
                    storage,
                    KnnSearchParams {
                        excluded,
                        embedding: &coi.point,
                        k_neighbors,
                        num_candidates: count,
                        published_after,
                        min_similarity: None,
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

async fn merge_knn_searchs(
    mut results: impl Stream<Item = Result<Vec<PersonalizedDocument>, Error>> + Unpin,
) -> (HashMap<DocumentId, PersonalizedDocument>, Vec<Error>) {
    let mut all_documents = HashMap::new();
    let mut errors = Vec::new();
    while let Some(result) = results.next().await {
        match result {
            Ok(documents) => {
                // the same document can be returned with different elastic scores, hence the
                // documents are deduplicated and only the highest score is retained for each
                for document in documents {
                    all_documents
                        .entry(document.id.clone())
                        .and_modify(|PersonalizedDocument { score, .. }| {
                            if *score < document.score {
                                *score = document.score;
                            }
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
        let documents = Search {
            interests: &[],
            excluded: &[],
            horizon: CoiConfig::default().horizon(),
            max_cois: PersonalizationConfig::default().max_cois_for_knn,
            count: 10,
            published_after: None,
        }
        .run_on(&storage)
        .await
        .unwrap();
        assert!(documents.is_empty());
    }
}
