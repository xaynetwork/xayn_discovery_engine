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

use xayn_discovery_engine_bert::{AveragePooler, SMBertConfig};
use xayn_discovery_engine_kpe::{Config as KpeConfig, RankedKeyPhrases};
use xayn_discovery_engine_providers::Market;

use crate::{
    coi::{
        config::Config as CoiSystemConfig,
        key_phrase::KeyPhrase,
        point::{NegativeCoi, PositiveCoi},
        CoiSystem,
    },
    embedding::Embedding,
    error::GenericError,
    ranker::{
        document::Document,
        system::{State, STATE_VERSION},
    },
    UserFeedback,
};

/// The ranker.
pub struct Ranker(super::system::Ranker);

impl Ranker {
    /// Creates a byte representation of the internal state of the ranker.
    pub fn serialize(&self) -> Result<Vec<u8>, GenericError> {
        self.0.serialize()
    }

    /// Computes the `SMBert` embedding of the given `sequence`.
    pub fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError> {
        self.0.compute_smbert(sequence)
    }

    /// Extracts the key phrases of the given `sequence`.
    pub fn extract_key_phrases(&self, sequence: &str) -> Result<RankedKeyPhrases, GenericError> {
        self.0.extract_key_phrases(sequence)
    }

    /// Ranks the given documents based on the learned user interests.
    pub fn rank(&mut self, items: &mut [impl Document]) {
        self.0.rank(items);
    }

    /// Logs the document view time and updates the user interests based on the given information.
    pub fn log_document_view_time(
        &mut self,
        user_feedback: UserFeedback,
        embedding: &Embedding,
        viewed: Duration,
    ) {
        self.0
            .log_document_view_time(user_feedback, embedding, viewed);
    }

    /// Logs the user reaction and updates the user interests based on the given information.
    pub fn log_user_reaction(
        &mut self,
        user_feedback: UserFeedback,
        title: &str,
        snippet: &str,
        embedding: &Embedding,
        market: &Market,
    ) {
        self.0
            .log_user_reaction(user_feedback, title, snippet, embedding, market);
    }

    /// Takes the top key phrases from the positive cois and market, sorted in descending relevance.
    pub fn take_key_phrases(&mut self, market: &Market, top: usize) -> Vec<KeyPhrase> {
        self.0.take_key_phrases(market, top)
    }

    /// Removes all key phrases associated to the markets.
    pub fn remove_key_phrases(&mut self, markets: &[Market]) {
        self.0.remove_key_phrases(markets);
    }

    /// Returns the positive cois.
    pub fn positive_cois(&self) -> &[PositiveCoi] {
        self.0.positive_cois()
    }

    /// Returns the negative cois.
    pub fn negative_cois(&self) -> &[NegativeCoi] {
        self.0.negative_cois()
    }

    /// Resets the AI state but not configurations.
    pub fn reset_ai(&mut self) {
        self.0.reset_ai();
    }
}

/// A builder for a [`Ranker`].
#[must_use]
pub struct Builder<'a, P> {
    smbert_config: SMBertConfig<'a, P>,
    coi_config: CoiSystemConfig,
    kpe_config: KpeConfig<'a>,
    state: State,
}

impl<'a> Builder<'a, AveragePooler> {
    /// Creates a builder from sub-configurations.
    pub fn from(smbert: SMBertConfig<'a, AveragePooler>, kpe: KpeConfig<'a>) -> Self {
        Builder {
            smbert_config: smbert,
            coi_config: CoiSystemConfig::default(),
            kpe_config: kpe,
            state: State::default(),
        }
    }

    /// Sets the serialized state to use.
    ///
    /// # Errors
    ///
    /// Fails if the state cannot be deserialized.
    pub fn with_serialized_state(mut self, bytes: impl AsRef<[u8]>) -> Result<Self, GenericError> {
        let bytes = bytes.as_ref();

        let state = match bytes[0] {
            version if version < STATE_VERSION => Ok(State::default()),
            STATE_VERSION => bincode::deserialize(&bytes[1..]).map_err(Into::into),
            version => Err(format!(
                "Unsupported serialized data. Found version {} expected {}",
                version, STATE_VERSION,
            )
            .into()),
        }
        .or_else(|e: GenericError|
                  // Serialized data could be the unversioned data we had before
                  bincode::deserialize(bytes).map(|user_interests|
                                                  State {
                                                      user_interests,
                                                      ..State::default()
                                                  }
                  ).map_err(|_| e))?;

        self.state = state;

        Ok(self)
    }

    /// Sets the [`CoiSystemConfig`] to use.
    pub fn with_coi_system_config(mut self, config: CoiSystemConfig) -> Self {
        self.coi_config = config;
        self
    }

    /// Creates a [`Ranker`].
    ///
    /// # Errors
    ///
    /// Fails if the `SMBert` or `KPE` cannot be initialized. For example because
    /// reading from a file failed or the bytes read are have an unexpected format.
    pub fn build(self) -> Result<Ranker, GenericError> {
        let smbert = xayn_discovery_engine_bert::Pipeline::from(self.smbert_config)?;
        let coi = CoiSystem::new(self.coi_config);
        let kpe = xayn_discovery_engine_kpe::Pipeline::from(self.kpe_config)?;

        Ok(Ranker(super::system::Ranker::new(
            smbert, coi, kpe, self.state,
        )))
    }
}
