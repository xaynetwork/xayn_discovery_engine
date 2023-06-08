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

use chrono::{DateTime, Utc};
use xayn_ai_bert::NormalizedEmbedding;

use crate::{
    compute_coi_relevances,
    config::Config,
    document::Document,
    point::{find_closest_coi_index, find_closest_coi_mut, Coi, Id},
    stats::compute_coi_decay_factor,
};

/// The center of interest (coi) system.
pub struct System {
    pub(super) config: Config,
}

impl System {
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Updates the view time of the [`Coi`] closest to the embedding.
    pub fn log_document_view_time(
        cois: &mut [Coi],
        embedding: &NormalizedEmbedding,
        viewed: Duration,
    ) {
        if let Some((coi, _)) = find_closest_coi_mut(cois, embedding) {
            coi.log_time(viewed);
        }
    }

    /// Updates the [`Coi`] closest to the embedding or creates a new one if it's too far away.
    pub fn log_user_reaction<'a>(
        &self,
        cois: &'a mut Vec<Coi>,
        embedding: &NormalizedEmbedding,
        time: DateTime<Utc>,
    ) -> &'a Coi {
        // If the given embedding's similarity to the CoI is above the threshold,
        // we adjust the position of the nearest CoI
        if let Some((index, similarity)) = find_closest_coi_index(cois, embedding) {
            if similarity >= self.config.threshold() {
                // normalization of the shifted coi is almost always possible
                if let Ok(coi) = cois[index].shift_point(embedding, self.config.shift_factor()) {
                    coi.log_reaction(time);
                    return &cois[index];
                }
            }
        }

        // If the embedding is too dissimilar, we create a new CoI instead
        cois.push(Coi::new(Id::new(), embedding.clone(), time));
        &cois[cois.len() - 1]
    }

    /// Computes the scores for all [`Document`]s wrt the [`Coi`]s.
    ///
    /// Each score ranges in the interval `[0., 1.]` if a [`Coi`] exists. The [coi weighting]
    /// outlines parts of the score calculation.
    ///
    /// [coi weighting]: https://xainag.atlassian.net/wiki/spaces/M2D/pages/2240708609/Discovery+engine+workflow#The-weighting-of-the-CoI
    pub fn score<D>(&self, documents: &[D], cois: &[Coi], time: DateTime<Utc>) -> Option<Vec<f32>>
    where
        D: Document,
    {
        documents
            .iter()
            .map(|document| {
                find_closest_coi_index(cois, document.embedding()).map(|(index, similarity)| {
                    let horizon = self.config.horizon();
                    let decay =
                        compute_coi_decay_factor(horizon, time, cois[index].stats.last_view);
                    let relevance = compute_coi_relevances(cois, horizon, time)[index];

                    (similarity * decay + relevance + 1.) / 4.
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::assert_approx_eq;

    use super::*;
    use crate::{document::tests::TestDocument, point::tests::create_cois};

    #[test]
    fn test_log_user_reaction_same_coi() {
        let now = Utc::now();
        let mut cois = create_cois([[1., 1., 1.], [10., 10., 10.], [20., 20., 20.]], now);
        let embedding = [2., 3., 4.].try_into().unwrap();
        let system = Config::default().build();

        let before = cois.clone();
        system.log_user_reaction(&mut cois, &embedding, now + chrono::Duration::seconds(1));

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
    fn test_log_user_reaction_new_coi() {
        let now = Utc::now();
        let mut cois = create_cois([[0., 1.]], now);
        let embedding = [1., 0.].try_into().unwrap();
        let system = Config::default().build();

        system.log_user_reaction(&mut cois, &embedding, now);

        assert_eq!(cois.len(), 2);
        assert_approx_eq!(f32, cois[0].point, [0., 1.,]);
        assert_approx_eq!(f32, cois[1].point, [1., 0.]);
    }

    #[test]
    fn test_log_document_view_time() {
        let mut cois = create_cois([[1., 2., 3.]], Utc::now());

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
    fn test_score() {
        let documents = vec![
            TestDocument::new(0, [3., 7., 0.].try_into().unwrap()),
            TestDocument::new(1, [1., 0., 0.].try_into().unwrap()),
            TestDocument::new(2, [1., 2., 0.].try_into().unwrap()),
            TestDocument::new(3, [5., 3., 0.].try_into().unwrap()),
        ];
        let now = Utc::now();
        let cois = create_cois([[1., 0., 0.], [4., 12., 2.]], now);

        let scores = Config::default()
            .build()
            .score(&documents, &cois, now)
            .unwrap();

        assert!(scores[3] < scores[2]);
        assert!(scores[2] < scores[0]);
        assert!(scores[0] < scores[1]);
    }

    #[test]
    fn test_score_no_cois() {
        let documents = vec![
            TestDocument::new(0, [0., 0., 0.].try_into().unwrap()),
            TestDocument::new(1, [0., 0., 0.].try_into().unwrap()),
            TestDocument::new(2, [0., 0., 0.].try_into().unwrap()),
        ];
        let scores = Config::default().build().score(&documents, &[], Utc::now());
        assert!(scores.is_none());
    }
}
