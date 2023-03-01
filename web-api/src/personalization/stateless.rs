// Copyright 2023 Xayn AG
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

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::Deserialize;
use xayn_ai_bert::NormalizedEmbedding;
use xayn_ai_coi::{CoiSystem, UserInterests};

use super::PersonalizationConfig;
use crate::{
    error::{common::HistoryTooSmall, warning::Warning},
    models::{DocumentId, DocumentTag},
    storage::{self, TagWeights},
    Error,
};

/// Represents a Users history passed to an endpoint.
///
/// The history is expected to be ordered from oldest to
/// newest entry, i.e. new entries are pushed to the end
/// of the history vec.
#[derive(Deserialize)]
#[serde(transparent)]
pub(super) struct InvalidatedHistory(Vec<InvalidatedHistoryEntry>);

#[derive(Deserialize)]
struct InvalidatedHistoryEntry {
    id: String,
    timestamp: Option<DateTime<Utc>>,
}

impl InvalidatedHistory {
    pub(super) async fn validate_and_load(
        self,
        storage: &impl storage::Document,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        let history = self.validate(config, warnings, Utc::now())?;
        Self::load(storage, history).await
    }

    fn validate(
        self,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
        time: DateTime<Utc>,
    ) -> Result<Vec<(DocumentId, DateTime<Utc>)>, Error> {
        if self.0.is_empty() {
            return Err(HistoryTooSmall.into());
        }
        let max_history_len = config.max_stateless_history_size;
        if self.0.len() > max_history_len {
            warnings.push(format!("history truncated, max length is {max_history_len}").into());
        }
        let mut most_recent_time = Utc::now();
        let mut history = self
            .0
            .into_iter()
            .rev()
            .take(max_history_len)
            .map(|unchecked| {
                let id = DocumentId::try_from(unchecked.id)?;
                let timestamp = unchecked.timestamp.unwrap_or(most_recent_time);
                if timestamp > most_recent_time {
                    warnings
                        .push(format!("inconsistent history ordering around document {id}").into());
                }
                most_recent_time = timestamp;
                Ok((id, timestamp))
            })
            .try_collect::<_, Vec<_>, Error>()?;

        history.reverse();
        Ok(history)
    }

    async fn load(
        storage: &impl storage::Document,
        history: Vec<(DocumentId, DateTime<Utc>)>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        let mut loaded =
            storage::Document::get_interacted(storage, history.iter().map(|(id, _)| id))
                .await?
                .into_iter()
                .map(|document| (document.id, (document.embedding, document.tags)))
                .collect::<HashMap<_, _>>();

        Ok(history
            .into_iter()
            // filter ignores documents which don't exist in our database (i.e. have
            // been deleted)
            .filter_map(|(id, timestamp)| {
                loaded.remove(&id).map(|(embedding, tags)| HistoryEntry {
                    id,
                    timestamp,
                    embedding,
                    tags,
                })
            })
            .collect())
    }
}

pub(super) struct HistoryEntry {
    pub(super) id: DocumentId,
    pub(super) timestamp: DateTime<Utc>,
    pub(super) embedding: NormalizedEmbedding,
    pub(super) tags: Vec<DocumentTag>,
}

/// Given an iterator over the history from _newest_ to oldest calculates user interests and tag weights.
pub(super) fn derive_interests_and_tag_weights<'a>(
    coi_system: &CoiSystem,
    history: impl IntoIterator<Item = &'a HistoryEntry>,
) -> (UserInterests, TagWeights) {
    let mut user_interests = UserInterests::default();
    let mut tag_weights = TagWeights::default();
    for entry in history {
        coi_system.log_positive_user_reaction(
            &mut user_interests.positive,
            &entry.embedding,
            entry.timestamp,
        );
        for tag in &entry.tags {
            *tag_weights.entry(tag.clone()).or_default() += 1;
        }
    }
    (user_interests, tag_weights)
}
