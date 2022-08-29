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
use xayn_discovery_engine_providers::Market;

use crate::{
    coi::{
        config::Config as CoiConfig,
        context::compute_scores_for_docs,
        key_phrase::{KeyPhrase, KeyPhrases},
        point::{find_closest_coi_mut, CoiPoint, NegativeCoi, PositiveCoi, UserInterests},
    },
    document::Document,
    embedding::Embedding,
    kps::config::Config as KpsConfig,
    nan_safe_f32_cmp_desc,
    GenericError,
};

/// The main system of the AI.
pub struct CoiSystem {
    pub(crate) coi_config: CoiConfig,
    pub(crate) kps_config: KpsConfig,
}

impl CoiSystem {
    pub fn coi_config(&self) -> &CoiConfig {
        &self.coi_config
    }

    pub fn kps_config(&self) -> &KpsConfig {
        &self.kps_config
    }

    /// Updates the view time of the positive coi closest to the embedding.
    pub fn log_document_view_time(
        cois: &mut [PositiveCoi],
        embedding: &Embedding,
        viewed: Duration,
    ) {
        log_document_view_time(cois, embedding, viewed);
    }

    /// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
    pub fn log_positive_user_reaction(
        &self,
        cois: &mut Vec<PositiveCoi>,
        market: &Market,
        key_phrases: &mut KeyPhrases,
        embedding: &Embedding,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
    ) {
        log_positive_user_reaction(
            cois,
            market,
            embedding,
            &self.coi_config,
            &self.kps_config,
            key_phrases,
            candidates,
            smbert,
        );
    }

    /// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
    pub fn log_negative_user_reaction(&self, cois: &mut Vec<NegativeCoi>, embedding: &Embedding) {
        log_negative_user_reaction(cois, embedding, &self.coi_config);
    }

    /// Takes the top key phrases from the positive cois and market, sorted in descending relevance.
    pub fn take_key_phrases(
        &self,
        cois: &[PositiveCoi],
        market: &Market,
        key_phrases: &mut KeyPhrases,
        top: usize,
    ) -> Vec<KeyPhrase> {
        key_phrases.take(
            cois,
            market,
            top,
            self.kps_config.horizon(),
            self.kps_config.penalty(),
            self.kps_config.gamma(),
        )
    }

    /// Removes all key phrases associated to the markets.
    pub fn remove_key_phrases(markets: &[Market], key_phrases: &mut KeyPhrases) {
        key_phrases.remove(markets);
    }

    /// Ranks the given documents in descending order of relevancy based on the
    /// learned user interests.
    pub fn rank(&self, documents: &mut [impl Document], user_interests: &UserInterests) {
        rank(documents, user_interests, &self.coi_config);
    }
}

/// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
#[allow(clippy::too_many_arguments)]
fn log_positive_user_reaction(
    cois: &mut Vec<PositiveCoi>,
    market: &Market,
    embedding: &Embedding,
    coi_config: &CoiConfig,
    kps_config: &KpsConfig,
    key_phrases: &mut KeyPhrases,
    candidates: &[String],
    smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
) {
    match find_closest_coi_mut(cois, embedding) {
        // If the given embedding's similarity to the CoI is above the threshold,
        // we adjust the position of the nearest CoI
        Some((coi, similarity)) if similarity >= coi_config.threshold() => {
            coi.shift_point(embedding, coi_config.shift_factor());
            coi.update_key_phrases(
                market,
                key_phrases,
                candidates,
                smbert,
                kps_config.max_key_phrases(),
                kps_config.gamma(),
            );
            coi.log_reaction();
        }

        // If the embedding is too dissimilar, we create a new CoI instead
        _ => {
            let coi = PositiveCoi::new(Uuid::new_v4(), embedding.clone());
            coi.update_key_phrases(
                market,
                key_phrases,
                candidates,
                smbert,
                kps_config.max_key_phrases(),
                kps_config.gamma(),
            );
            cois.push(coi);
        }
    }
}

/// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
fn log_negative_user_reaction(
    cois: &mut Vec<NegativeCoi>,
    embedding: &Embedding,
    coi_config: &CoiConfig,
) {
    match find_closest_coi_mut(cois, embedding) {
        Some((coi, similarity)) if similarity >= coi_config.threshold() => {
            coi.shift_point(embedding, coi_config.shift_factor());
            coi.log_reaction();
        }
        _ => cois.push(NegativeCoi::new(Uuid::new_v4(), embedding.clone())),
    }
}

/// Updates the view time of the positive coi closest to the embedding.
fn log_document_view_time(cois: &mut [PositiveCoi], embedding: &Embedding, viewed: Duration) {
    if let Some((coi, _)) = find_closest_coi_mut(cois, embedding) {
        coi.log_time(viewed);
    }
}

#[instrument(skip_all)]
fn rank(documents: &mut [impl Document], user_interests: &UserInterests, config: &CoiConfig) {
    if documents.len() < 2 {
        return;
    }

    if let Ok(scores_for_docs) = compute_scores_for_docs(documents, user_interests, config) {
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

#[cfg(test)]
mod tests {
    use ndarray::arr1;

    use super::*;
    use crate::{
        coi::point::tests::{create_neg_cois, create_pos_cois},
        document::{tests::TestDocument, DocumentId},
    };

    #[test]
    fn test_update_coi_update_point() {
        let mut cois = create_pos_cois(&[[1., 1., 1.], [10., 10., 10.], [20., 20., 20.]]);
        let mut key_phrases = KeyPhrases::default();
        let embedding = arr1(&[2., 3., 4.]).into();
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        let last_view = cois[0].stats.last_view;
        log_positive_user_reaction(
            &mut cois,
            &Market::new("aa", "AA"),
            &embedding,
            &coi_config,
            &kps_config,
            &mut key_phrases,
            &[],
            |_| unreachable!(),
        );

        assert_eq!(cois.len(), 3);
        assert_eq!(cois[0].point, arr1(&[1.1, 1.2, 1.3]));
        assert_eq!(cois[1].point, arr1(&[10., 10., 10.]));
        assert_eq!(cois[2].point, arr1(&[20., 20., 20.]));

        assert_eq!(cois[0].stats.view_count, 2);
        assert!(cois[0].stats.last_view > last_view);
    }

    #[test]
    fn test_update_coi_under_similarity_threshold_adds_new_coi() {
        let mut cois = create_pos_cois(&[[0., 1.]]);
        let mut key_phrases = KeyPhrases::default();
        let embedding = arr1(&[1., 0.]).into();
        let coi_config = CoiConfig::default();
        let kps_config = KpsConfig::default();

        log_positive_user_reaction(
            &mut cois,
            &Market::new("aa", "AA"),
            &embedding,
            &coi_config,
            &kps_config,
            &mut key_phrases,
            &[],
            |_| unreachable!(),
        );

        assert_eq!(cois.len(), 2);
        assert_eq!(cois[0].point, arr1(&[0., 1.,]));
        assert_eq!(cois[1].point, arr1(&[1., 0.]));
    }

    #[test]
    fn test_log_negative_user_reaction_last_view() {
        let mut cois = create_neg_cois(&[[1., 2., 3.]]);
        let embedding = arr1(&[1., 2., 4.]).into();
        let config = CoiConfig::default();

        let last_view = cois[0].last_view;
        log_negative_user_reaction(&mut cois, &embedding, &config);

        assert_eq!(cois.len(), 1);
        assert!(cois[0].last_view > last_view);
    }

    #[test]
    fn test_log_document_view_time() {
        let mut cois = create_pos_cois(&[[1., 2., 3.]]);

        log_document_view_time(
            &mut cois,
            &arr1(&[1., 2., 4.]).into(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(10), cois[0].stats.view_time);

        log_document_view_time(
            &mut cois,
            &arr1(&[1., 2., 4.]).into(),
            Duration::from_secs(10),
        );
        assert_eq!(Duration::from_secs(20), cois[0].stats.view_time);
    }

    #[test]
    fn test_rank() {
        let mut documents = vec![
            TestDocument::new(0, arr1(&[3., 7., 0.]), "2000-01-01 00:00:03"),
            TestDocument::new(1, arr1(&[1., 0., 0.]), "2000-01-01 00:00:02"),
            TestDocument::new(2, arr1(&[1., 2., 0.]), "2000-01-01 00:00:01"),
            TestDocument::new(3, arr1(&[5., 3., 0.]), "2000-01-01 00:00:00"),
        ];
        let user_interests = UserInterests {
            positive: create_pos_cois(&[[1., 0., 0.], [4., 12., 2.]]),
            negative: create_neg_cois(&[[-100., -10., 0.]]),
        };
        let config = CoiConfig::default()
            .with_min_positive_cois(2)
            .unwrap()
            .with_min_negative_cois(1);

        rank(&mut documents, &user_interests, &config);

        assert_eq!(documents[0].id(), DocumentId::from_u128(1));
        assert_eq!(documents[1].id(), DocumentId::from_u128(3));
        assert_eq!(documents[2].id(), DocumentId::from_u128(2));
        assert_eq!(documents[3].id(), DocumentId::from_u128(0));
    }

    #[test]
    fn test_rank_no_user_interests() {
        let mut documents = vec![
            TestDocument::new(0, arr1(&[0., 0., 0.]), "2000-01-01 00:00:03"),
            TestDocument::new(1, arr1(&[0., 0., 0.]), "2000-01-01 00:00:01"),
            TestDocument::new(2, arr1(&[0., 0., 0.]), "2000-01-01 00:00:02"),
        ];
        let user_interests = UserInterests::default();
        let config = CoiConfig::default().with_min_positive_cois(1).unwrap();

        rank(&mut documents, &user_interests, &config);

        assert_eq!(documents[0].id(), DocumentId::from_u128(0));
        assert_eq!(documents[1].id(), DocumentId::from_u128(2));
        assert_eq!(documents[2].id(), DocumentId::from_u128(1));
    }

    #[test]
    fn test_rank_no_documents() {
        let mut documents = Vec::<TestDocument>::new();
        let user_interests = UserInterests::default();
        let config = CoiConfig::default();

        rank(&mut documents, &user_interests, &config);

        assert!(documents.is_empty());
    }
}
