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

use displaydoc::Display;
use thiserror::Error;

use crate::{
    coi::{
        compute_coi_decay_factor,
        compute_coi_relevances,
        config::Config,
        find_closest_coi,
        point::{CoiPoint, NegativeCoi, PositiveCoi, UserInterests},
    },
    embedding::Embedding,
    ranker::document::Document,
    utils::system_time_now,
    CoiId,
    DocumentId,
};

#[derive(Error, Debug, Display)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Error {
    /// Not enough cois
    NotEnoughCois,
    /// Failed to find the closest cois
    FailedToFindTheClosestCois,
}

struct ClosestPositiveCoi {
    /// The ID of the closest positive centre of interest
    id: CoiId,
    /// Similarity to the closest positive centre of interest
    similarity: f32,
    last_view: SystemTime,
}

impl ClosestPositiveCoi {
    fn new(
        embedding: &Embedding,
        positive_user_interests: &[PositiveCoi],
    ) -> Result<Option<Self>, Error> {
        let this = find_closest_coi(positive_user_interests, embedding).map(|(coi, similarity)| {
            ClosestPositiveCoi {
                id: coi.id(),
                similarity,
                last_view: coi.stats.last_view,
            }
        });

        if !positive_user_interests.is_empty() && this.is_none() {
            Err(Error::FailedToFindTheClosestCois)
        } else {
            Ok(this)
        }
    }

    fn score(
        &self,
        positive_user_interests: &[PositiveCoi],
        horizon: Duration,
        now: SystemTime,
    ) -> f32 {
        let decay = compute_coi_decay_factor(horizon, now, self.last_view);
        let index = positive_user_interests
            .iter()
            .position(|coi| coi.id() == self.id)
            .unwrap();
        let relevance = compute_coi_relevances(positive_user_interests, horizon, now)[index];

        self.similarity * decay + relevance
    }
}

struct ClosestNegativeCoi {
    /// Similarity to closest negative centre of interest
    similarity: f32,
    last_view: SystemTime,
}

impl ClosestNegativeCoi {
    fn new(
        embedding: &Embedding,
        negative_user_interests: &[NegativeCoi],
    ) -> Result<Option<Self>, Error> {
        let this = find_closest_coi(negative_user_interests, embedding).map(|(coi, similarity)| {
            ClosestNegativeCoi {
                similarity,
                last_view: coi.last_view,
            }
        });

        if !negative_user_interests.is_empty() && this.is_none() {
            Err(Error::FailedToFindTheClosestCois)
        } else {
            Ok(this)
        }
    }

    fn score(&self, horizon: Duration, now: SystemTime) -> f32 {
        let decay = compute_coi_decay_factor(horizon, now, self.last_view);

        self.similarity * decay
    }
}

struct ClosestCois {
    positive: Option<ClosestPositiveCoi>,
    negative: Option<ClosestNegativeCoi>,
}

impl ClosestCois {
    fn new(embedding: &Embedding, user_interests: &UserInterests) -> Result<Self, Error> {
        let positive = ClosestPositiveCoi::new(embedding, &user_interests.positive)?;
        let negative = ClosestNegativeCoi::new(embedding, &user_interests.negative)?;

        Ok(Self { positive, negative })
    }

    fn score(
        &self,
        positive_user_interests: &[PositiveCoi],
        horizon: Duration,
        now: SystemTime,
    ) -> f32 {
        let positive = self
            .positive
            .as_ref()
            .map(|positive| positive.score(positive_user_interests, horizon, now))
            .unwrap_or_default();
        let negative = self
            .negative
            .as_ref()
            .map(|negative| negative.score(horizon, now))
            .unwrap_or_default();

        (positive - negative).clamp(f32::MIN, f32::MAX) // avoid positive or negative infinity
    }
}

fn compute_score_for_embedding(
    embedding: &Embedding,
    user_interests: &UserInterests,
    horizon: Duration,
    now: SystemTime,
) -> Result<f32, Error> {
    ClosestCois::new(embedding, user_interests)
        .map(|cois| cois.score(&user_interests.positive, horizon, now))
}

fn has_enough_cois(
    user_interests: &UserInterests,
    min_positive_cois: usize,
    min_negative_cois: usize,
) -> bool {
    user_interests.positive.len() >= min_positive_cois
        && user_interests.negative.len() >= min_negative_cois
}

/// Computes the score for all documents based on the given information.
///
/// <https://xainag.atlassian.net/wiki/spaces/M2D/pages/2240708609/Discovery+engine+workflow#The-weighting-of-the-CoI>
/// outlines parts of the score calculation.
///
/// # Errors
/// Fails if the required number of positive or negative cois is not present.
pub(super) fn compute_score_for_docs(
    documents: &[impl Document],
    user_interests: &UserInterests,
    config: &Config,
) -> Result<HashMap<DocumentId, f32>, Error> {
    if !has_enough_cois(
        user_interests,
        config.min_positive_cois(),
        config.min_negative_cois(),
    ) {
        return Err(Error::NotEnoughCois);
    }

    let now = system_time_now();
    documents
        .iter()
        .map(|document| {
            let score = compute_score_for_embedding(
                document.smbert_embedding(),
                user_interests,
                config.horizon(),
                now,
            )?;
            Ok((document.id(), score))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ndarray::arr1;
    use xayn_discovery_engine_test_utils::assert_approx_eq;

    use crate::{
        coi::{create_neg_cois, create_pos_cois},
        utils::SECONDS_PER_DAY_F32,
    };

    use super::*;

    #[test]
    fn test_has_enough_cois() {
        let user_interests = UserInterests::default();

        assert!(has_enough_cois(&user_interests, 0, 0));
        assert!(!has_enough_cois(&user_interests, 1, 0));
        assert!(!has_enough_cois(&user_interests, 0, 1));
    }

    #[test]
    fn test_compute_score_for_embedding() {
        let embedding = arr1(&[1., 4., 4.]).into();

        let epoch = SystemTime::UNIX_EPOCH;
        let now = epoch + Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32);

        let mut positive = create_pos_cois(&[[62., 55., 11.], [76., 30., 80.]]);
        positive[0].stats.last_view -= Duration::from_secs_f32(0.5 * SECONDS_PER_DAY_F32);
        positive[1].stats.last_view -= Duration::from_secs_f32(1.5 * SECONDS_PER_DAY_F32);

        let mut negative = create_neg_cois(&[[6., 61., 6.]]);
        negative[0].last_view = epoch;
        let user_interests = UserInterests { positive, negative };

        let horizon = Duration::from_secs_f32(2. * SECONDS_PER_DAY_F32);

        let score = compute_score_for_embedding(&embedding, &user_interests, horizon, now).unwrap();

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
        let embedding = arr1(&[0., 0., 0.]).into();
        let horizon = Duration::from_secs_f32(SECONDS_PER_DAY_F32);

        let res = compute_score_for_embedding(
            &embedding,
            &UserInterests::default(),
            horizon,
            system_time_now(),
        );

        assert_approx_eq!(f32, res.unwrap(), f32::default());
    }
}
