use std::time::Duration;

use displaydoc::Display;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    coi::{
        config::Config,
        point::{
            find_closest_coi,
            find_closest_coi_mut,
            CoiPoint,
            NegativeCoi,
            PositiveCoi,
            UserInterests,
        },
        relevance::RelevanceMap,
        utils::classify_documents_based_on_user_feedback,
        CoiId,
    },
    // data::document_data::{CoiComponent, DocumentDataWithCoi, DocumentDataWithSMBert},
    embedding::{
        smbert::SMBert,
        utils::{Embedding, MINIMUM_COSINE_SIMILARITY},
    },
    DocumentHistory,
    Error,
};

use super::key_phrase::KeyPhrase;

#[derive(Error, Debug, Display)]
pub(crate) enum CoiSystemError {
    /// No CoI could be found for the given embedding
    NoCoi,
    /// No matching documents could be found
    NoMatchingDocuments,
}

pub(crate) struct CoiSystem {
    pub(crate) config: Config,
    smbert: SMBert,
}

impl CoiSystem {
    /// Creates a new centre of interest system.
    pub(crate) fn new(config: Config, smbert: SMBert) -> Self {
        Self { config, smbert }
    }

    /// Updates the view time of the positive coi closest to the embedding.
    pub(crate) fn log_document_view_time(
        &mut self,
        cois: &mut [PositiveCoi],
        embedding: &Embedding,
        viewed: Duration,
    ) {
        log_document_view_time(cois, embedding, viewed);
    }

    /// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
    pub(crate) fn log_positive_user_reaction(
        &mut self,
        cois: &mut Vec<PositiveCoi>,
        relevances: &mut RelevanceMap,
        embedding: &Embedding,
        smbert: impl Fn(&str) -> Result<Embedding, Error> + Sync,
        candidates: &[String],
    ) {
        log_positive_user_reaction(
            cois,
            embedding,
            &self.config,
            relevances,
            smbert,
            candidates,
        );
    }

    /// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
    pub(crate) fn log_negative_user_reaction(
        &self,
        cois: &mut Vec<NegativeCoi>,
        embedding: &Embedding,
    ) {
        log_negative_user_reaction(cois, embedding, &self.config);
    }

    /// Selects the top key phrases from the positive cois, sorted in descending relevance.
    pub(crate) fn select_top_key_phrases(
        &mut self,
        cois: &[PositiveCoi],
        relevances: &mut RelevanceMap,
        top: usize,
    ) -> Vec<KeyPhrase> {
        relevances.select_top_key_phrases(cois, top, self.config.horizon(), self.config.penalty())
    }
}

/// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
fn log_positive_user_reaction(
    cois: &mut Vec<PositiveCoi>,
    embedding: &Embedding,
    config: &Config,
    relevances: &mut RelevanceMap,
    smbert: impl Fn(&str) -> Result<Embedding, Error> + Sync,
    candidates: &[String],
) {
    match find_closest_coi_mut(cois, embedding) {
        // If the given embedding's similarity to the CoI is above the threshold,
        // we adjust the position of the nearest CoI
        Some((coi, similarity)) if similarity >= config.threshold() => {
            coi.shift_point(embedding, config.shift_factor());
            coi.select_key_phrases(
                relevances,
                candidates,
                smbert,
                config.max_key_phrases(),
                config.gamma(),
            );
            coi.log_reaction();
        }

        // If the embedding is too dissimilar, we create a new CoI instead
        _ => {
            let coi = PositiveCoi::new(Uuid::new_v4(), embedding.clone());
            coi.select_key_phrases(
                relevances,
                candidates,
                smbert,
                config.max_key_phrases(),
                config.gamma(),
            );
            cois.push(coi);
        }
    }
}

/// Updates the negative coi closest to the embedding or creates a new one if it's too far away.
fn log_negative_user_reaction(cois: &mut Vec<NegativeCoi>, embedding: &Embedding, config: &Config) {
    match find_closest_coi_mut(cois, embedding) {
        Some((coi, similarity)) if similarity >= config.threshold() => {
            coi.shift_point(embedding, config.shift_factor());
            coi.log_reaction();
        }
        _ => cois.push(NegativeCoi::new(Uuid::new_v4(), embedding.clone())),
    }
}

/// Updates the negative cois based on the documents data.
fn log_document_view_time(cois: &mut [PositiveCoi], embedding: &Embedding, viewed: Duration) {
    if let Some((coi, _)) = find_closest_coi_mut(cois, embedding) {
        coi.log_time(viewed);
    }
}

#[cfg(test)]
mod tests {
    use ndarray::{arr1, FixedInitializer};
    use std::f32::NAN;

    use super::*;
    use crate::{
        coi::{
            utils::tests::{
                create_document_history,
                create_neg_cois,
                create_pos_cois,
            },
            CoiId,
        },
        data::{
            document::{DocumentId, Relevance, UserFeedback},
        },
        utils::to_vec_of_ref_of,
    };
    use test_utils::assert_approx_eq;

    #[test]
    fn test_update_coi_add_point() {
        let mut cois = create_pos_cois(&[[1., 0., 0.], [1., 0.2, 1.], [0.5, 0.5, 0.1]]);
        let mut relevances = RelevanceMap::default();
        let embedding = arr1(&[1.91, 73.78, 72.35]).into();
        let config = Config::default();

        let (closest, similarity) = find_closest_coi(&cois, &embedding).unwrap();

        assert_eq!(closest.point, arr1(&[0.5, 0.5, 0.1]));
        assert_approx_eq!(f32, similarity, 0.610_772_5);
        assert!(config.threshold() >= similarity);

        log_positive_user_reaction(
            &mut cois,
            &embedding,
            &config,
            &mut relevances,
            |_| unreachable!(),
            &[],
        );
        assert_eq!(cois.len(), 4);
    }

    #[test]
    fn test_update_coi_update_point() {
        let mut cois = create_pos_cois(&[[1., 1., 1.], [10., 10., 10.], [20., 20., 20.]]);
        let mut relevances = RelevanceMap::default();
        let embedding = arr1(&[2., 3., 4.]).into();
        let config = Config::default();

        let last_view_before = cois[0].stats.last_view;

        log_positive_user_reaction(
            &mut cois,
            &embedding,
            &config,
            &mut relevances,
            |_| unreachable!(),
            &[],
        );

        assert_eq!(cois.len(), 3);
        assert_eq!(cois[0].point, arr1(&[1.1, 1.2, 1.3]));
        assert_eq!(cois[1].point, arr1(&[10., 10., 10.]));
        assert_eq!(cois[2].point, arr1(&[20., 20., 20.]));

        assert_eq!(cois[0].stats.view_count, 2);
        assert!(cois[0].stats.last_view > last_view_before);
    }

    #[test]
    fn test_update_coi_under_similarity_threshold_adds_new_coi() {
        let mut cois = create_pos_cois(&[[0., 1.]]);
        let mut relevances = RelevanceMap::default();
        let embedding = arr1(&[1., 0.]).into();
        let config = Config::default();

        log_positive_user_reaction(
            &mut cois,
            &embedding,
            &config,
            &mut relevances,
            |_| unreachable!(),
            &[],
        );

        assert_eq!(cois.len(), 2);
        assert_eq!(cois[0].point, arr1(&[0., 1.,]));
        assert_eq!(cois[1].point, arr1(&[1., 0.]));
    }

    #[test]
    fn test_log_negative_user_reaction_last_view() {
        let mut cois = create_neg_cois(&[[1., 2., 3.]]);
        let config = Config::default();
        let before = cois[0].last_view;
        log_negative_user_reaction(&mut cois, &arr1(&[1., 2., 4.]).into(), &config);
        assert!(cois[0].last_view > before);
        assert_eq!(cois.len(), 1);
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
}
