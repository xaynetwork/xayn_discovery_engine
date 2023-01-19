// Copyright 2021 Xayn AG
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

use tracing::instrument;
use xayn_ai_bert::NormalizedEmbedding;

use crate::{
    config::Config,
    context,
    context::compute_scores_for_docs,
    document::Document,
    id::CoiId,
    point::{
        find_closest_coi_index,
        find_closest_coi_mut,
        CoiPoint,
        NegativeCoi,
        PositiveCoi,
        UserInterests,
    },
};

/// The center of interest (coi) system.
pub struct System {
    pub(super) config: Config,
}

impl System {
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Updates the view time of the positive coi closest to the embedding.
    pub fn log_document_view_time(
        cois: &mut [PositiveCoi],
        embedding: &NormalizedEmbedding,
        viewed: Duration,
    ) {
        if let Some((coi, _)) = find_closest_coi_mut(cois, embedding) {
            coi.log_time(viewed);
        }
    }

    /// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
    pub fn log_positive_user_reaction<'a>(
        &self,
        cois: &'a mut Vec<PositiveCoi>,
        embedding: &NormalizedEmbedding,
    ) -> &'a PositiveCoi {
        // If the given embedding's similarity to the CoI is above the threshold,
        // we adjust the position of the nearest CoI
        if let Some((index, similarity)) = find_closest_coi_index(cois, embedding) {
            if similarity >= self.config.threshold() {
                // normalization of the shifted coi is almost always possible
                if let Ok(coi) = cois[index].shift_point(embedding, self.config.shift_factor()) {
                    coi.log_reaction();
                    return &cois[index];
                }
            }
        }

        // If the embedding is too dissimilar, we create a new CoI instead
        cois.push(PositiveCoi::new(CoiId::new(), embedding.clone()));
        &cois[cois.len() - 1]
    }

    /// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
    pub fn log_negative_user_reaction(
        &self,
        cois: &mut Vec<NegativeCoi>,
        embedding: &NormalizedEmbedding,
    ) {
        if let Some((coi, similarity)) = find_closest_coi_mut(cois, embedding) {
            if similarity >= self.config.threshold() {
                if let Ok(coi) = coi.shift_point(embedding, self.config.shift_factor()) {
                    coi.log_reaction();
                    return;
                }
            }
        }

        cois.push(NegativeCoi::new(CoiId::new(), embedding.clone()));
    }

    #[instrument(skip_all)]
    /// Return the score of each document given the interests of the user.
    pub fn score<D>(
        &self,
        documents: &[D],
        user_interests: &UserInterests,
    ) -> Result<HashMap<D::Id, f32>, context::Error>
    where
        D: Document,
    {
        compute_scores_for_docs(documents, user_interests, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use xayn_ai_test_utils::assert_approx_eq;

    use super::*;
    use crate::{
        document::tests::{DocumentId, TestDocument},
        point::tests::{create_neg_cois, create_pos_cois},
        utils,
    };

    #[test]
    fn test_log_positive_user_reaction_same_coi() {
        let mut cois = create_pos_cois([[1., 1., 1.], [10., 10., 10.], [20., 20., 20.]]);
        let embedding = [2., 3., 4.].try_into().unwrap();
        let system = Config::default().build();

        let before = cois.clone();
        system.log_positive_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 3);
        assert_approx_eq!(
            f32,
            cois[0].point,
            [0.558_521_4, 0.577_149_87, 0.595_778_35],
        );
        assert_approx_eq!(f32, cois[1].point, before[1].point);
        assert_approx_eq!(f32, cois[2].point, before[2].point);

        assert_eq!(cois[0].stats.view_count, 2);
        assert!(cois[0].stats.last_view > before[0].stats.last_view);
    }

    #[test]
    fn test_log_positive_user_reaction_new_coi() {
        let mut cois = create_pos_cois([[0., 1.]]);
        let embedding = [1., 0.].try_into().unwrap();
        let system = Config::default().build();

        system.log_positive_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 2);
        assert_approx_eq!(f32, cois[0].point, [0., 1.,]);
        assert_approx_eq!(f32, cois[1].point, [1., 0.]);
    }

    #[test]
    fn test_log_negative_user_reaction_last_view() {
        let mut cois = create_neg_cois([[1., 2., 3.]]);
        let embedding = [1., 2., 4.].try_into().unwrap();
        let system = Config::default().build();

        let last_view = cois[0].last_view;
        system.log_negative_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 1);
        assert!(cois[0].last_view > last_view);
    }

    #[test]
    fn test_log_document_view_time() {
        let mut cois = create_pos_cois([[1., 2., 3.]]);

        System::log_document_view_time(
            &mut cois,
            &[1., 2., 4.].try_into().unwrap(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(10), cois[0].stats.view_time);

        System::log_document_view_time(
            &mut cois,
            &[1., 2., 4.].try_into().unwrap(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(20), cois[0].stats.view_time);
    }

    #[test]
    fn test_rank() {
        let mut documents = vec![
            TestDocument::new(0, [3., 7., 0.].try_into().unwrap()),
            TestDocument::new(1, [1., 0., 0.].try_into().unwrap()),
            TestDocument::new(2, [1., 2., 0.].try_into().unwrap()),
            TestDocument::new(3, [5., 3., 0.].try_into().unwrap()),
        ];
        let user_interests = UserInterests {
            positive: create_pos_cois([[1., 0., 0.], [4., 12., 2.]]),
            negative: create_neg_cois([[-100., -10., 0.]]),
        };
        let system = Config::default()
            .with_min_positive_cois(2)
            .unwrap()
            .with_min_negative_cois(1)
            .build();

        let scores = system.score(&documents, &user_interests).unwrap();
        utils::rank(&mut documents, &scores);

        assert_eq!(*documents[0].id(), DocumentId::mocked(1));
        assert_eq!(*documents[1].id(), DocumentId::mocked(3));
        assert_eq!(*documents[2].id(), DocumentId::mocked(2));
        assert_eq!(*documents[3].id(), DocumentId::mocked(0));
    }

    #[test]
    fn test_rank_no_user_interests() {
        let documents = vec![
            TestDocument::new(0, [0., 0., 0.].try_into().unwrap()),
            TestDocument::new(1, [0., 0., 0.].try_into().unwrap()),
            TestDocument::new(2, [0., 0., 0.].try_into().unwrap()),
        ];
        let user_interests = UserInterests::default();
        let system = Config::default().with_min_positive_cois(1).unwrap().build();

        assert!(system.score(&documents, &user_interests).is_err());
    }
}
