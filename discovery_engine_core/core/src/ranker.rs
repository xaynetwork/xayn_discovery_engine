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

use uuid::Uuid;

use xayn_ai::{
    ranker::{Embedding, KeyPhrase},
    DocumentId,
    UserFeedback,
};

use crate::{
    document::{Document, Id, TimeSpent, UserReacted, UserReaction},
    engine::GenericError,
};

/// Provides a method for ranking a slice of [`Document`] items.
pub trait Ranker {
    /// Performs the ranking of [`Document`] items.
    //TODO check exactly how/when ranking fails.
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError>;

    /// Logs the time a user spent on a document.
    fn log_document_view_time(&mut self, time_spent: &TimeSpent) -> Result<(), GenericError>;

    /// Logs a user's interaction.
    fn log_user_reaction(&mut self, reaction: &UserReacted) -> Result<(), GenericError>;

    /// Selects the top key phrases from the positive cois, sorted in descending relevance.
    fn select_top_key_phrases(&mut self, top: usize) -> Vec<KeyPhrase>;

    /// Serializes the state of the `Ranker`.
    fn serialize(&self) -> Result<Vec<u8>, GenericError>;

    /// Computes the S-mBert embedding of the given `sequence`.
    //TODO check exactly how/when ranking fails.
    fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError>;
}

impl Ranker for xayn_ai::ranker::Ranker {
    // TODO the ranking implementation we currently use can't fail as it will
    //      fallback to sort by score on error. We could make `Ranker.rank` non
    //      erroring for now, if we combine it with a few similar changes we can
    //      remove most error cases in `update_stack`, especially if we indicate
    //      but not return an error if fetching new documents fails, in which case
    //      we could make it error less.
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError> {
        self.rank(items).map_err(Into::into)
    }

    // TODO this can't fail, it should not return a result
    fn log_document_view_time(&mut self, time_spent: &TimeSpent) -> Result<(), GenericError> {
        self.log_document_view_time(
            time_spent.reaction.into(),
            &time_spent.smbert_embedding,
            time_spent.time,
        );
        Ok(())
    }

    // TODO this can't fail, it should not return a result
    fn log_user_reaction(&mut self, reaction: &UserReacted) -> Result<(), GenericError> {
        self.log_user_reaction(
            reaction.reaction.into(),
            &reaction.snippet,
            &reaction.smbert_embedding,
        );
        Ok(())
    }

    fn select_top_key_phrases(&mut self, top: usize) -> Vec<KeyPhrase> {
        self.select_top_key_phrases(top)
    }

    fn serialize(&self) -> Result<Vec<u8>, GenericError> {
        self.serialize().map_err(Into::into)
    }

    fn compute_smbert(&self, sequence: &str) -> Result<Embedding, GenericError> {
        self.compute_smbert(sequence).map_err(Into::into)
    }
}

impl xayn_ai::ranker::Document for Document {
    fn id(&self) -> DocumentId {
        self.id.into()
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }

    fn score(&self) -> Option<f32> {
        self.resource.score
    }

    fn rank(&self) -> usize {
        self.resource.rank
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
