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

use xayn_ai::{ranker::Embedding, DocumentId, UserFeedback};

use crate::{
    document::{Document, Id, TimeSpent, UserReacted, UserReaction},
    engine::GenericError,
};

/// Provides a method for ranking a slice of [`Document`] items.
pub trait Ranker {
    /// Performs the ranking of [`Document`] items.
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError>;

    /// Learn from the time a user spent on a document.
    fn time_logged(&mut self, time_logged: &TimeSpent) -> Result<(), GenericError>;

    /// Learn from a user's interaction.
    fn user_reacted(&mut self, reaction: &UserReacted) -> Result<(), GenericError>;
}

impl Ranker for xayn_ai::ranker::Ranker {
    fn rank(&mut self, items: &mut [Document]) -> Result<(), GenericError> {
        self.rank(items).map_err(Into::into)
    }

    fn time_logged(&mut self, time_logged: &TimeSpent) -> Result<(), GenericError> {
        self.log_document_view_time(
            (&time_logged.reaction).into(),
            &time_logged.smbert,
            time_logged.seconds,
        );
        Ok(())
    }

    fn user_reacted(&mut self, reaction: &UserReacted) -> Result<(), GenericError> {
        self.log_user_reaction(
            (&reaction.reaction).into(),
            &reaction.snippet,
            &reaction.smbert,
        );
        Ok(())
    }
}

impl xayn_ai::ranker::Document for Document {
    fn id(&self) -> DocumentId {
        self.id.into()
    }

    fn smbert_embedding(&self) -> &Embedding {
        &self.smbert_embedding
    }
}

impl From<Id> for DocumentId {
    fn from(id: Id) -> Self {
        Self(id.0)
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
