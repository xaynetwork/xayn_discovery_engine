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

use displaydoc::Display;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use xayn_discovery_engine_bert::SMBert;
use xayn_discovery_engine_kpe::{Pipeline as KPE, RankedKeyPhrases};
use xayn_discovery_engine_providers::Market;

use crate::{
    coi::{
        config::Config,
        key_phrase::{KeyPhrase, KeyPhrases},
        point::{NegativeCoi, PositiveCoi, UserInterests},
        CoiSystem,
    },
    embedding::Embedding,
    error::GenericError,
    nan_safe_f32_cmp_desc,
    ranker::{
        context::{compute_score_for_docs, Error as ContextError},
        document::{Document, UserFeedback},
    },
    utils::serialize_with_version,
};

#[derive(Error, Debug, Display)]
pub(crate) enum RankerError {
    /// No user interests are known.
    Context(#[from] ContextError),
}

pub(super) const STATE_VERSION: u8 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct State {
    /// The learned user interests.
    pub(super) user_interests: UserInterests,

    /// Key phrases.
    pub(super) key_phrases: KeyPhrases,
}

/// The Ranker.
pub(crate) struct Ranker {
    /// SMBert system.
    smbert: SMBert,
    /// CoI system.
    coi: CoiSystem,
    /// Key phrase extraction system.
    kpe: KPE,
    state: State,
}

impl Ranker {
    /// Creates a new `Ranker`.
    pub(super) fn new(smbert: SMBert, coi: CoiSystem, kpe: KPE, state: State) -> Self {
        Self {
            smbert,
            coi,
            kpe,
            state,
        }
    }

    /// Creates a byte representation of the internal state of the ranker.
    pub(crate) fn serialize(&self) -> Result<Vec<u8>, GenericError> {
        serialize_with_version(&self.state, STATE_VERSION)
    }

    /// Computes the `SMBert` embedding of the given `sequence`.
    pub(crate) fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError> {
        self.smbert.run(sequence).map_err(Into::into)
    }

    /// Extracts the key phrases of the given `sequence`.
    pub(crate) fn extract_key_phrases(
        &self,
        sequence: &str,
    ) -> Result<RankedKeyPhrases, GenericError> {
        self.kpe.run(sequence).map_err(Into::into)
    }

    /// Ranks the given documents in descending order of relevancy based on the
    /// learned user interests.
    pub(crate) fn rank(&mut self, documents: &mut [impl Document]) {
        rank(documents, &self.state.user_interests, &self.coi.config);
    }

    /// Logs the document view time and updates the user interests based on the given information.
    pub(crate) fn log_document_view_time(
        &mut self,
        user_feedback: UserFeedback,
        embedding: &Embedding,
        viewed: Duration,
    ) {
        if let UserFeedback::Relevant | UserFeedback::NotGiven = user_feedback {
            CoiSystem::log_document_view_time(
                &mut self.state.user_interests.positive,
                embedding,
                viewed,
            );
        }
    }

    /// Logs the user reaction and updates the user interests based on the given information.
    pub(crate) fn log_user_reaction(
        &mut self,
        user_feedback: UserFeedback,
        title: &str,
        snippet: &str,
        embedding: &Embedding,
        market: &Market,
    ) {
        match user_feedback {
            UserFeedback::Relevant => {
                let smbert = &self.smbert;
                let key_phrases = self
                    .kpe
                    .run(snippet)
                    .or_else(|_| self.kpe.run(format!("{title} {snippet}")))
                    .map_or_else(
                        #[allow(clippy::if_not_else)]
                        |_| {
                            vec![if !title.is_empty() {
                                title.to_string()
                            } else {
                                snippet.to_string()
                            }]
                        },
                        Into::into,
                    );
                self.coi.log_positive_user_reaction(
                    &mut self.state.user_interests.positive,
                    market,
                    &mut self.state.key_phrases,
                    embedding,
                    key_phrases.as_slice(),
                    |words| smbert.run(words).map_err(Into::into),
                );
            }
            UserFeedback::Irrelevant => self
                .coi
                .log_negative_user_reaction(&mut self.state.user_interests.negative, embedding),
            UserFeedback::NotGiven => {}
        }
    }

    /// Takes the top key phrases from the positive cois and market, sorted in descending relevance.
    pub(crate) fn take_key_phrases(&mut self, market: &Market, top: usize) -> Vec<KeyPhrase> {
        self.coi.take_key_phrases(
            &self.state.user_interests.positive,
            market,
            &mut self.state.key_phrases,
            top,
        )
    }

    /// Removes all key phrases associated to the markets.
    pub(crate) fn remove_key_phrases(&mut self, markets: &[Market]) {
        CoiSystem::remove_key_phrases(markets, &mut self.state.key_phrases);
    }

    /// Returns the positive cois.
    pub(crate) fn positive_cois(&self) -> &[PositiveCoi] {
        self.state.user_interests.positive.as_slice()
    }

    /// Returns the negative cois.
    pub(crate) fn negative_cois(&self) -> &[NegativeCoi] {
        self.state.user_interests.negative.as_slice()
    }

    /// Resets the AI state but not configurations.
    pub(crate) fn reset_ai(&mut self) {
        self.state = State::default();
    }
}

fn rank(documents: &mut [impl Document], user_interests: &UserInterests, config: &Config) {
    if documents.len() < 2 {
        return;
    }

    if let Ok(score_for_docs) = compute_score_for_docs(documents, user_interests, config) {
        documents.sort_unstable_by(|this, other| {
            nan_safe_f32_cmp_desc(
                score_for_docs.get(&this.id()).unwrap(),
                score_for_docs.get(&other.id()).unwrap(),
            )
        });
    } else {
        documents
            .sort_unstable_by(|this, other| other.date_published().cmp(&this.date_published()));
    }
}

#[cfg(test)]
mod tests {
    use ndarray::arr1;

    use crate::{
        coi::{create_neg_cois, create_pos_cois},
        ranker::document::TestDocument,
        DocumentId,
    };

    use super::*;

    #[test]
    fn test_rank() {
        let mut documents = vec![
            TestDocument::new(0, arr1(&[3., 7., 0.]), "2000-01-01 00:00:03"),
            TestDocument::new(1, arr1(&[1., 0., 0.]), "2000-01-01 00:00:02"),
            TestDocument::new(2, arr1(&[1., 2., 0.]), "2000-01-01 00:00:01"),
            TestDocument::new(3, arr1(&[5., 3., 0.]), "2000-01-01 00:00:00"),
        ];

        let config = Config::default()
            .with_min_positive_cois(2)
            .unwrap()
            .with_min_negative_cois(1)
            .unwrap();
        let positive = create_pos_cois(&[[1., 0., 0.], [4., 12., 2.]]);
        let negative = create_neg_cois(&[[-100., -10., 0.]]);

        let user_interests = UserInterests { positive, negative };

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

        let config = Config::default().with_min_positive_cois(1).unwrap();

        rank(&mut documents, &UserInterests::default(), &config);

        assert_eq!(documents[0].id(), DocumentId::from_u128(0));
        assert_eq!(documents[1].id(), DocumentId::from_u128(2));
        assert_eq!(documents[2].id(), DocumentId::from_u128(1));
    }

    #[test]
    fn test_rank_no_documents() {
        let mut documents = Vec::<TestDocument>::new();
        rank(
            &mut documents,
            &UserInterests::default(),
            &Config::default(),
        );
        assert!(documents.is_empty());
    }
}
