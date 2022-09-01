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

use xayn_discovery_engine_providers::Market;

use crate::{
    coi::point::PositiveCoi,
    embedding::Embedding,
    kps::{
        config::Config,
        key_phrase::{KeyPhrase, KeyPhrases},
    },
    GenericError,
};

/// The key phrase selection (kps) system.
pub struct System {
    pub(super) config: Config,
}

impl System {
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Updates the positive coi closest to the embedding or creates a new one if it's too far away.
    #[allow(clippy::too_many_arguments)]
    pub fn log_positive_user_reaction(
        &self,
        system: &crate::coi::system::System,
        cois: &mut Vec<PositiveCoi>,
        embedding: &Embedding,
        market: &Market,
        key_phrases: &mut KeyPhrases,
        candidates: &[String],
        smbert: impl Fn(&str) -> Result<Embedding, GenericError> + Sync,
    ) {
        system
            .log_positive_user_reaction(cois, embedding)
            .update_key_phrases(
                market,
                key_phrases,
                candidates,
                smbert,
                self.config.max_key_phrases(),
                self.config.gamma(),
            );
    }

    /// Takes the top key phrases from the positive cois and market, sorted in descending relevance.
    pub fn take_key_phrases(
        &self,
        system: &crate::coi::system::System,
        cois: &[PositiveCoi],
        market: &Market,
        key_phrases: &mut KeyPhrases,
        top: usize,
    ) -> Vec<KeyPhrase> {
        key_phrases.take(
            cois,
            market,
            top,
            system.config().horizon(),
            self.config.penalty(),
            self.config.gamma(),
        )
    }

    /// Removes all key phrases associated to the markets.
    pub fn remove_key_phrases(markets: &[Market], key_phrases: &mut KeyPhrases) {
        key_phrases.remove(markets);
    }
}
