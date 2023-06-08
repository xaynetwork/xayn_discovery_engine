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
    models::{DocumentId, PersonalizedDocument},
    storage::{self, KnnSearchParams},
    Error,
};

/// KNN search based on Centers of Interest.
pub(super) struct CoiSearch<I, J> {
    pub(super) interests: I,
    pub(super) excluded: J,
    pub(super) horizon: Duration,
    pub(super) max_cois: usize,
    pub(super) count: usize,
    pub(super) published_after: Option<DateTime<Utc>>,
    pub(super) time: DateTime<Utc>,
}

impl<'a, I, J> CoiSearch<I, J>
where
    I: IntoIterator,
    <I as IntoIterator>::IntoIter: Clone + Iterator<Item = &'a Coi>,
    J: IntoIterator,
    <J as IntoIterator>::IntoIter: Clone + Iterator<Item = &'a DocumentId>,
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
            .sorted_by(|(_, w1), (_, w2)| w1.total_cmp(w2).reverse())
            .take(self.max_cois)
            .collect_vec();

        let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

        let excluded = self.excluded.into_iter();
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
                let count = (weight * self.count as f32).ceil() as usize;
                let num_candidates = self.count.max(count);
                storage::Document::get_by_embedding(
                    storage,
                    KnnSearchParams {
                        excluded: excluded.clone(),
                        embedding: &coi.point,
                        count,
                        num_candidates,
                        published_after: self.published_after,
                        min_similarity: None,
                        query: None,
                        time: self.time,
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
        let documents = CoiSearch {
            interests: &[],
            excluded: &[],
            horizon: CoiConfig::default().horizon(),
            max_cois: PersonalizationConfig::default().max_cois_for_knn,
            count: 10,
            published_after: None,
            time: Utc::now(),
        }
        .run_on(&storage)
        .await
        .unwrap();
        assert!(documents.is_empty());
    }
}
