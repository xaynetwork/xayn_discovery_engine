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

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use xayn_ai_bert::NormalizedEmbedding;

use crate::{
    config::Config,
    document::Document,
    point::{find_closest_coi, find_closest_coi_index, NegativeCoi, PositiveCoi},
    stats::{compute_coi_decay_factor, compute_coi_relevances},
    utils::system_time_now,
};

fn compute_score_for_closest_positive_coi(
    embedding: &NormalizedEmbedding,
    cois: &[PositiveCoi],
    horizon: Duration,
    now: SystemTime,
) -> Option<f32> {
    find_closest_coi_index(cois, embedding).map(|(index, similarity)| {
        let decay = compute_coi_decay_factor(horizon, now, cois[index].stats.last_view);
        let relevance = compute_coi_relevances(cois, horizon, now)[index];
        similarity * decay + relevance
    })
}

fn compute_score_for_closest_negative_coi(
    embedding: &NormalizedEmbedding,
    cois: &[NegativeCoi],
    horizon: Duration,
    now: SystemTime,
) -> Option<f32> {
    find_closest_coi(cois, embedding).map(|(coi, similarity)| {
        let decay = compute_coi_decay_factor(horizon, now, coi.last_view);
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
    /// Checks if all user interests are empty.
    pub fn is_empty(&self) -> bool {
        self.positive.is_empty() && self.negative.is_empty()
    }

    fn has_enough_cois(&self, min_positive: usize, min_negative: usize) -> bool {
        self.positive.len() >= min_positive && self.negative.len() >= min_negative
    }

    fn compute_score_for_embedding(
        &self,
        embedding: &NormalizedEmbedding,
        horizon: Duration,
        now: SystemTime,
    ) -> Option<f32> {
        match (
            compute_score_for_closest_positive_coi(embedding, &self.positive, horizon, now),
            compute_score_for_closest_negative_coi(embedding, &self.negative, horizon, now),
        ) {
            (Some(positive), Some(negative)) => Some(positive - negative),
            (Some(positive), None) => Some(positive),
            (None, Some(negative)) => Some(-negative),
            (None, None) => None,
        }
    }

    /// Computes the scores for all documents based on the given information.
    ///
    /// <https://xainag.atlassian.net/wiki/spaces/M2D/pages/2240708609/Discovery+engine+workflow#The-weighting-of-the-CoI>
    /// outlines parts of the score calculation.
    pub(crate) fn compute_scores_for_docs<D>(
        &self,
        documents: &[D],
        config: &Config,
    ) -> Option<HashMap<D::Id, f32>>
    where
        D: Document,
    {
        if !self.has_enough_cois(config.min_positive_cois(), config.min_negative_cois()) {
            return None;
        }

        let now = system_time_now();
        let scores = documents
            .iter()
            .map(|document| {
                let score = self.compute_score_for_embedding(
                    document.bert_embedding(),
                    config.horizon(),
                    now,
                ).unwrap(/* checked that some coi exists */);
                (document.id().clone(), score)
            })
            .collect();

        Some(scores)
    }
}

#[cfg(test)]
mod tests {
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;
    use crate::{
        point::tests::{create_neg_cois, create_pos_cois},
        utils::SECONDS_PER_DAY_F32,
    };

    #[test]
    fn test_has_enough_cois() {
        let user_interests = UserInterests::default();
        assert!(user_interests.has_enough_cois(0, 0));
        assert!(!user_interests.has_enough_cois(1, 0));
        assert!(!user_interests.has_enough_cois(0, 1));
    }

    #[test]
    fn test_compute_score_for_embedding() {
        let epoch = SystemTime::UNIX_EPOCH;
        let now = epoch + Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32);
        let mut positive = create_pos_cois([[62., 55., 11.], [76., 30., 80.]]);
        positive[0].stats.last_view -= Duration::from_secs_f32(0.5 * SECONDS_PER_DAY_F32);
        positive[1].stats.last_view -= Duration::from_secs_f32(1.5 * SECONDS_PER_DAY_F32);

        let mut negative = create_neg_cois([[6., 61., 6.]]);
        negative[0].last_view = epoch;
        let user_interests = UserInterests { positive, negative };

        let horizon = Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32);

        let score = user_interests
            .compute_score_for_embedding(&[1., 4., 4.].try_into().unwrap(), horizon, now)
            .unwrap();

        let pos_similarity = 0.785_516_44;
        let pos_decay = 0.999_999_34;
        let neg_similarity = 0.774_465_6;
        let neg_decay = 0.;
        let relevance = 0.499_999_67;
        let expected = pos_similarity * pos_decay + relevance - neg_similarity * neg_decay;
        assert_approx_eq!(f32, score, expected, epsilon = 1e-6);
    }

    #[test]
    fn test_compute_score_for_embedding_no_cois() {
        let horizon = Duration::from_secs_f32(SECONDS_PER_DAY_F32);
        let score = UserInterests::default().compute_score_for_embedding(
            &[0., 0., 0.].try_into().unwrap(),
            horizon,
            system_time_now(),
        );
        assert!(score.is_none());
    }
}
