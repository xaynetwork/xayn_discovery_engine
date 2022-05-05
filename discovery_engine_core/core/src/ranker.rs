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

use chrono::NaiveDateTime;
use uuid::Uuid;

use xayn_ai::{
    ranker::{Embedding, KeyPhrase, NegativeCoi, PositiveCoi},
    DocumentId,
    UserFeedback,
};

use crate::{
    document::{Document, Id, TimeSpent, UserReacted, UserReaction},
    engine::GenericError,
};

#[cfg(test)]
use mockall::automock;

/// Provides a method for ranking a slice of [`Document`] items.
#[cfg_attr(test, automock)]
pub trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError>;

    /// Logs the time a user spent on a document.
    fn log_document_view_time(&mut self, time_spent: &TimeSpent) -> Result<(), GenericError>;

    /// Logs a user's interaction.
    fn log_user_reaction(&mut self, reaction: &UserReacted) -> Result<(), GenericError>;

    /// Takes the top key phrases from the positive cois, sorted in descending relevance.
    fn take_key_phrases(&mut self, top: usize) -> Vec<KeyPhrase>;

    /// Serializes the state of the `Ranker`.
    fn serialize(&self) -> Result<Vec<u8>, GenericError>;

    /// Computes the S-mBert embedding of the given `sequence`.
    fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError>;

    /// Returns the positive cois.
    fn positive_cois(&self) -> &[PositiveCoi];

    /// Returns the negative cois.
    fn negative_cois(&self) -> &[NegativeCoi];
}

impl Ranker for xayn_ai::ranker::Ranker {
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError> {
        self.rank(items).map_err(Into::into)
    }

    fn log_document_view_time(&mut self, time_spent: &TimeSpent) -> Result<(), GenericError> {
        self.log_document_view_time(
            time_spent.reaction.into(),
            &time_spent.smbert_embedding,
            time_spent.time,
        );
        Ok(())
    }

    fn log_user_reaction(&mut self, reaction: &UserReacted) -> Result<(), GenericError> {
        self.log_user_reaction(
            reaction.reaction.into(),
            &reaction.snippet,
            &reaction.smbert_embedding,
            &reaction.market,
        );
        Ok(())
    }

    fn take_key_phrases(&mut self, top: usize) -> Vec<KeyPhrase> {
        self.take_key_phrases(top)
    }

    fn serialize(&self) -> Result<Vec<u8>, GenericError> {
        self.serialize().map_err(Into::into)
    }

    fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError> {
        self.compute_smbert(sequence).map_err(Into::into)
    }

    fn positive_cois(&self) -> &[PositiveCoi] {
        self.positive_cois()
    }

    fn negative_cois(&self) -> &[NegativeCoi] {
        self.negative_cois()
    }
}

impl xayn_ai::ranker::Document for Document {
    fn id(&self) -> DocumentId {
        self.id.into()
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }

    fn date_published(&self) -> NaiveDateTime {
        self.resource.date_published
    }
}

impl From<Id> for DocumentId {
    fn from(id: Id) -> Self {
        Self(Uuid::from(id))
    }
}

impl From<UserReaction> for UserFeedback {
    fn from(reaction: UserReaction) -> Self {
        match reaction {
            UserReaction::Neutral => UserFeedback::NotGiven,
            UserReaction::Positive => UserFeedback::Relevant,
            UserReaction::Negative => UserFeedback::Irrelevant,
        }
    }
}
