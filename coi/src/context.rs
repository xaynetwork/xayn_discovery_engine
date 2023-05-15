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

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use xayn_ai_bert::NormalizedEmbedding;

use crate::{
    config::Config,
    document::Document,
    point::{find_closest_coi, find_closest_coi_index, NegativeCoi, PositiveCoi},
    stats::{compute_coi_decay_factor, compute_coi_relevances},
};

/// The score ranges in the interval `[-1., 3.]` if a positive coi exists.
fn compute_score_for_closest_positive_coi(
    embedding: &NormalizedEmbedding,
    cois: &[PositiveCoi],
    horizon: Duration,
    time: DateTime<Utc>,
) -> Option<f32> {
    find_closest_coi_index(cois, embedding).map(|(index, similarity)| {
        let decay = compute_coi_decay_factor(horizon, time, cois[index].stats.last_view);
        let relevance = compute_coi_relevances(cois, horizon, time)[index];
        similarity * decay + relevance
    })
}

/// The score ranges in the interval `[-1., 1.]` if a negative coi exists.
fn compute_score_for_closest_negative_coi(
    embedding: &NormalizedEmbedding,
    cois: &[NegativeCoi],
    horizon: Duration,
    time: DateTime<Utc>,
) -> Option<f32> {
    find_closest_coi(cois, embedding).map(|(coi, similarity)| {
        let decay = compute_coi_decay_factor(horizon, time, coi.last_view);
        similarity * decay
    })
}

/// The `CoI`s of a user.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct UserInterests {
    pub positive: Vec<PositiveCoi>,
    pub negative: Vec<NegativeCoi>,
}

impl UserInterests {
    pub fn has_enough(&self, config: &Config) -> bool {
        self.positive.len() >= config.min_positive_cois()
            && self.negative.len() >= config.min_negative_cois()
    }

    /// The score ranges in the interval `[0., 1.]` if a coi exists.
    fn compute_score_for_embedding(
        &self,
        embedding: &NormalizedEmbedding,
        horizon: Duration,
        time: DateTime<Utc>,
    ) -> Option<f32> {
        match (
            compute_score_for_closest_positive_coi(embedding, &self.positive, horizon, time),
            compute_score_for_closest_negative_coi(embedding, &self.negative, horizon, time),
        ) {
            (Some(positive), Some(negative)) => Some((positive - negative + 2.) / 6.),
            (Some(positive), None) => Some((positive + 1.) / 4.),
            (None, Some(negative)) => Some((-negative + 1.) / 2.),
            (None, None) => None,
        }
    }

    /// Computes the scores for all documents.
    ///
    /// Each score ranges in the interval `[0., 1.]` if a coi exists. The [coi weighting] outlines
    /// parts of the score calculation.
    ///
    /// [coi weighting]: https://xainag.atlassian.net/wiki/spaces/M2D/pages/2240708609/Discovery+engine+workflow#The-weighting-of-the-CoI
    pub(crate) fn compute_scores_for_docs<D>(
        &self,
        documents: &[D],
        config: &Config,
        time: DateTime<Utc>,
    ) -> Option<Vec<f32>>
    where
        D: Document,
    {
        documents
            .iter()
            .map(|document| {
                self.compute_score_for_embedding(document.bert_embedding(), config.horizon(), time)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use xayn_test_utils::assert_approx_eq;

    use super::*;
    use crate::point::tests::{create_neg_cois, create_pos_cois};

    #[test]
    fn test_has_enough() {
        let config = Config::default();
        let cois = UserInterests::default();
        assert!(!cois.has_enough(&config));
        let now = Utc::now();
        let cois = UserInterests {
            positive: create_pos_cois([[1., 2., 3.]], now),
            negative: Vec::new(),
        };
        assert!(cois.has_enough(&config));
        let cois = UserInterests {
            positive: Vec::new(),
            negative: create_neg_cois([[1., 2., 3.]], now),
        };
        assert!(!cois.has_enough(&config));
    }

    #[test]
    fn test_compute_score_for_embedding() {
        let now = Utc::now();
        let mut positive = create_pos_cois([[62., 55., 11.], [76., 30., 80.]], now);
        positive[0].stats.last_view = now - Duration::hours(12);
        positive[1].stats.last_view = now - Duration::hours(36);
        let mut negative = create_neg_cois([[6., 61., 6.]], now);
        negative[0].last_view = now - Duration::days(1);
        let cois = UserInterests { positive, negative };

        let embedding = [1., 4., 4.].try_into().unwrap();
        let horizon = Duration::days(2).to_std().unwrap();
        let score = cois
            .compute_score_for_embedding(&embedding, horizon, now)
            .unwrap();
        assert_approx_eq!(
            f32,
            score,
            // positive[1]: similarity * decay + relevance
            (0.785_516_44 * 0.231_573_88 + 0.115_786_94
            // negative[0]: similarity * decay
            - 0.774_465_7 * 0.475_020_83
            // normalize
            + 2.)
                / 6.,
        );
    }

    #[test]
    fn test_compute_score_for_embedding_no_cois() {
        let horizon = Duration::days(1).to_std().unwrap();
        let score = UserInterests::default().compute_score_for_embedding(
            &[0., 0., 0.].try_into().unwrap(),
            horizon,
            Utc::now(),
        );
        assert!(score.is_none());
    }
}
