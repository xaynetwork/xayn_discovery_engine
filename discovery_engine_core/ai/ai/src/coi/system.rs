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

use std::time::Duration;

use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::{
    coi::{
        config::Config,
        context::compute_scores_for_docs,
        point::{
            find_closest_coi_index,
            find_closest_coi_mut,
            CoiPoint,
            NegativeCoi,
            PositiveCoi,
            UserInterests,
        },
    },
    document::Document,
    embedding::Embedding,
    nan_safe_f32_cmp_desc,
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
        embedding: &Embedding,
        viewed: Duration,
    ) {
        if let Some((coi, _)) = find_closest_coi_mut(cois, embedding) {
            coi.log_time(viewed);
        }
    }

    /// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
    #[allow(clippy::missing_panics_doc)]
    pub fn log_positive_user_reaction<'a>(
        &self,
        cois: &'a mut Vec<PositiveCoi>,
        embedding: &Embedding,
    ) -> &'a PositiveCoi {
        match find_closest_coi_index(cois, embedding) {
            // If the given embedding's similarity to the CoI is above the threshold,
            // we adjust the position of the nearest CoI
            Some((index, similarity)) if similarity >= self.config.threshold() => cois
                .get_mut(index)
                .unwrap(/* index is in bounds */)
                .shift_point(embedding, self.config.shift_factor())
                .log_reaction(),
            // If the embedding is too dissimilar, we create a new CoI instead
            _ => {
                cois.push(PositiveCoi::new(Uuid::new_v4(), embedding.clone()));
                cois.last().unwrap(/* coi is just pushed */)
            }
        }
    }

    /// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
    pub fn log_negative_user_reaction(&self, cois: &mut Vec<NegativeCoi>, embedding: &Embedding) {
        match find_closest_coi_mut(cois, embedding) {
            Some((coi, similarity)) if similarity >= self.config.threshold() => {
                coi.shift_point(embedding, self.config.shift_factor());
                coi.log_reaction();
            }
            _ => cois.push(NegativeCoi::new(Uuid::new_v4(), embedding.clone())),
        }
    }

    /// Ranks the given documents in descending order of relevancy based on the
    /// learned user interests.
    #[instrument(skip_all)]
    pub fn rank(&self, documents: &mut [impl Document], user_interests: &UserInterests) {
        if documents.len() < 2 {
            return;
        }

        if let Ok(scores_for_docs) =
            compute_scores_for_docs(documents, user_interests, &self.config)
        {
            for (document, score) in &scores_for_docs {
                debug!(%document, score);
            }
            documents.sort_unstable_by(|this, other| {
                nan_safe_f32_cmp_desc(
                    scores_for_docs.get(&this.id()).unwrap(),
                    scores_for_docs.get(&other.id()).unwrap(),
                )
            });
        } else {
            info!(message = "no scores could be computed");
            documents
                .sort_unstable_by(|this, other| other.date_published().cmp(&this.date_published()));
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use ndarray::arr1;

    use super::*;
    use crate::{
        coi::point::tests::{create_neg_cois, create_pos_cois},
        document::{tests::TestDocument, DocumentId},
    };

    #[test]
    fn test_log_positive_user_reaction_same_coi() {
        let mut cois = create_pos_cois(&[[1., 1., 1.], [10., 10., 10.], [20., 20., 20.]]);
        let embedding = arr1(&[2., 3., 4.]).into();
        let system = Config::default().build();

        let last_view = cois[0].stats.last_view;
        system.log_positive_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 3);
        assert_eq!(cois[0].point, arr1(&[1.1, 1.2, 1.3]));
        assert_eq!(cois[1].point, arr1(&[10., 10., 10.]));
        assert_eq!(cois[2].point, arr1(&[20., 20., 20.]));

        assert_eq!(cois[0].stats.view_count, 2);
        assert!(cois[0].stats.last_view > last_view);
    }

    #[test]
    fn test_log_positive_user_reaction_new_coi() {
        let mut cois = create_pos_cois(&[[0., 1.]]);
        let embedding = arr1(&[1., 0.]).into();
        let system = Config::default().build();

        system.log_positive_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 2);
        assert_eq!(cois[0].point, arr1(&[0., 1.,]));
        assert_eq!(cois[1].point, arr1(&[1., 0.]));
    }

    #[test]
    fn test_log_negative_user_reaction_last_view() {
        let mut cois = create_neg_cois(&[[1., 2., 3.]]);
        let embedding = arr1(&[1., 2., 4.]).into();
        let system = Config::default().build();

        let last_view = cois[0].last_view;
        system.log_negative_user_reaction(&mut cois, &embedding);

        assert_eq!(cois.len(), 1);
        assert!(cois[0].last_view > last_view);
    }

    #[test]
    fn test_log_document_view_time() {
        let mut cois = create_pos_cois(&[[1., 2., 3.]]);

        System::log_document_view_time(
            &mut cois,
            &arr1(&[1., 2., 4.]).into(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(10), cois[0].stats.view_time);

        System::log_document_view_time(
            &mut cois,
            &arr1(&[1., 2., 4.]).into(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(20), cois[0].stats.view_time);
    }

    #[test]
    fn test_rank() {
        let mut documents = vec![
            TestDocument::new(0, arr1(&[3., 7., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 3)),
            TestDocument::new(1, arr1(&[1., 0., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 2)),
            TestDocument::new(2, arr1(&[1., 2., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 1)),
            TestDocument::new(3, arr1(&[5., 3., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 0)),
        ];
        let user_interests = UserInterests {
            positive: create_pos_cois(&[[1., 0., 0.], [4., 12., 2.]]),
            negative: create_neg_cois(&[[-100., -10., 0.]]),
        };
        let system = Config::default()
            .with_min_positive_cois(2)
            .unwrap()
            .with_min_negative_cois(1)
            .build();

        system.rank(&mut documents, &user_interests);

        assert_eq!(documents[0].id(), DocumentId::from_u128(1));
        assert_eq!(documents[1].id(), DocumentId::from_u128(3));
        assert_eq!(documents[2].id(), DocumentId::from_u128(2));
        assert_eq!(documents[3].id(), DocumentId::from_u128(0));
    }

    #[test]
    fn test_rank_no_user_interests() {
        let mut documents = vec![
            TestDocument::new(0, arr1(&[0., 0., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 3)),
            TestDocument::new(1, arr1(&[0., 0., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 1)),
            TestDocument::new(2, arr1(&[0., 0., 0.]), Utc.ymd(2000, 1, 1).and_hms(0, 0, 2)),
        ];
        let user_interests = UserInterests::default();
        let system = Config::default().with_min_positive_cois(1).unwrap().build();

        system.rank(&mut documents, &user_interests);

        assert_eq!(documents[0].id(), DocumentId::from_u128(0));
        assert_eq!(documents[1].id(), DocumentId::from_u128(2));
        assert_eq!(documents[2].id(), DocumentId::from_u128(1));
    }

    #[test]
    fn test_rank_no_documents() {
        let mut documents = Vec::<TestDocument>::new();
        let user_interests = UserInterests::default();
        let system = Config::default().build();

        system.rank(&mut documents, &user_interests);

        assert!(documents.is_empty());
    }
}
