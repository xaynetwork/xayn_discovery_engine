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
use xayn_ai_coi::{Coi, CoiSystem};

use super::{routes::UnvalidatedDocumentOrSnippetId, PersonalizationConfig};
use crate::{
    error::{common::HistoryTooSmall, warning::Warning},
    models::{DocumentTags, SnippetForInteraction, SnippetId, SnippetOrDocumentId},
    storage::{self, TagWeights},
    Error,
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedHistoryEntry {
    id: UnvalidatedDocumentOrSnippetId,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
}

#[derive(PartialEq, Debug)]
pub(super) struct HistoryEntry {
    pub(super) id: SnippetOrDocumentId,
    pub(super) timestamp: DateTime<Utc>,
}

/// Validates given history.
///
/// The history is expected to be ordered from oldest to
/// newest entry, i.e. new entries are pushed to the end
/// of the history vec.
///
/// The returned history is also ordered from oldest to newest.
pub(super) fn validate_history(
    history: Vec<UnvalidatedHistoryEntry>,
    config: &PersonalizationConfig,
    warnings: &mut Vec<Warning>,
    time: DateTime<Utc>,
    allow_empty_history: bool,
) -> Result<Vec<HistoryEntry>, Error> {
    if !allow_empty_history && history.is_empty() {
        return Err(HistoryTooSmall.into());
    }
    let max_history_len = config.max_stateless_history_size;
    if history.len() > max_history_len {
        warnings.push(format!("history truncated, max length is {max_history_len}").into());
    }
    let mut most_recent_time = time;
    let mut history = history
        .into_iter()
        .rev()
        .take(max_history_len)
        .map(|unchecked| {
            let id = unchecked.id.validate()?;
            let timestamp = unchecked.timestamp.unwrap_or(most_recent_time);
            if timestamp > most_recent_time {
                let document_id = id.document_id();
                let snippet_idx = id.snippet_idx();
                warnings.push(format!("inconsistent history ordering around document {document_id} snippet {snippet_idx:?}").into());
            }
            most_recent_time = timestamp;
            Ok(HistoryEntry { id, timestamp })
        })
        .try_collect::<_, Vec<_>, Error>()?;

    history.reverse();
    Ok(history)
}

/// Trims history to only contain the `max_len` newest documents.
pub(super) fn trim_history(mut history: Vec<HistoryEntry>, max_len: usize) -> Vec<HistoryEntry> {
    if let Some(surplus) = history.len().checked_sub(max_len) {
        history.drain(..surplus);
    }
    history
}

/// Enriches the history with data loaded from the database.
pub(super) async fn load_history(
    storage: &impl storage::Document,
    history: Vec<HistoryEntry>,
) -> Result<Vec<LoadedHistoryEntry>, Error> {
    let history = history
        .into_iter()
        .map(|entry| {
            // TODO[pmk/soon] properly support history of documents with multiple snippets
            let id = match entry.id {
                SnippetOrDocumentId::SnippetId(id) => id,
                SnippetOrDocumentId::DocumentId(id) => SnippetId::new(id, 0),
            };
            (id, entry.timestamp)
        })
        .collect::<HashMap<_, _>>();

    let loaded = storage::Document::get_snippets_for_interaction(storage, history.keys())
        .await?
        .into_iter();

    Ok(loaded
        .into_iter()
        .map(
            |SnippetForInteraction {
                 id,
                 embedding,
                 tags,
             }| {
                let timestamp = history[&id];
                LoadedHistoryEntry {
                    timestamp,
                    embedding,
                    tags,
                }
            },
        )
        .collect())
}

pub(super) struct LoadedHistoryEntry {
    pub(super) timestamp: DateTime<Utc>,
    pub(super) embedding: NormalizedEmbedding,
    pub(super) tags: DocumentTags,
}

/// Given an iterator over the history from oldest to newest calculates user interests and tag weights.
pub(super) fn derive_interests_and_tag_weights<'a>(
    coi_system: &CoiSystem,
    history: impl IntoIterator<Item = &'a LoadedHistoryEntry>,
) -> (Vec<Coi>, TagWeights) {
    let mut interests = Vec::new();
    let mut tag_weights = TagWeights::new();
    for entry in history {
        coi_system.log_user_reaction(&mut interests, &entry.embedding, entry.timestamp);
        for tag in &entry.tags {
            *tag_weights.entry(tag.clone()).or_default() += 1;
        }
    }
    (interests, tag_weights)
}

#[doc(hidden)]
pub fn bench_derive_interests(
    coi_system: &CoiSystem,
    history: Vec<(DateTime<Utc>, NormalizedEmbedding)>,
) {
    // small allocation overhead, but we don't have to expose a lot of private items
    let history = history
        .into_iter()
        .map(|(timestamp, embedding)| LoadedHistoryEntry {
            timestamp,
            embedding,
            tags: DocumentTags::default(),
        })
        .collect_vec();
    derive_interests_and_tag_weights(coi_system, &history);
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone};
    use xayn_ai_bert::Embedding1;
    use xayn_ai_coi::CoiConfig;
    use xayn_test_utils::error::Panic;

    use super::*;
    use crate::models::DocumentTag;

    #[test]
    fn test_validating_empty_history_fails() {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let config = PersonalizationConfig::default();
        let mut warnings = Vec::new();
        let res = validate_history(vec![], &config, &mut warnings, now, false);
        assert!(res.is_err());
        assert!(warnings.is_empty());
        let res = validate_history(vec![], &config, &mut warnings, now, true);
        assert!(res.is_ok());
        assert!(warnings.is_empty());
    }

    fn unvalidated_doc_id(id: &str) -> UnvalidatedDocumentOrSnippetId {
        UnvalidatedDocumentOrSnippetId::DocumentId(id.to_owned())
    }

    fn doc_id(id: &str) -> SnippetOrDocumentId {
        SnippetOrDocumentId::DocumentId(id.try_into().unwrap())
    }

    #[test]
    fn test_validating_to_large_history_warns() -> Result<(), Panic> {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let config = PersonalizationConfig {
            max_stateless_history_size: 1,
            ..Default::default()
        };
        let mut warnings = Vec::new();

        validate_history(
            vec![UnvalidatedHistoryEntry {
                id: unvalidated_doc_id("doc-1"),
                timestamp: Some(now - Duration::days(1)),
            }],
            &config,
            &mut warnings,
            now,
            true,
        )?;
        assert!(warnings.is_empty());

        let documents = validate_history(
            vec![
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-1"),
                    timestamp: Some(now - Duration::days(2)),
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-2"),
                    timestamp: Some(now - Duration::days(1)),
                },
            ],
            &config,
            &mut warnings,
            now,
            true,
        )?;

        assert_eq!(warnings.len(), 1);
        assert_eq!(
            documents,
            vec![HistoryEntry {
                id: SnippetOrDocumentId::DocumentId("doc-2".try_into()?),
                timestamp: now - Duration::days(1)
            }]
        );

        Ok(())
    }

    #[test]
    fn test_history_gaps_are_filled_in() -> Result<(), Panic> {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let config = PersonalizationConfig::default();
        let mut warnings = Vec::new();

        let documents = validate_history(
            vec![
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-1"),
                    timestamp: Some(now - Duration::days(2)),
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-2"),
                    timestamp: None,
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-3"),
                    timestamp: None,
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-4"),
                    timestamp: Some(now - Duration::days(1)),
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-5"),
                    timestamp: None,
                },
            ],
            &config,
            &mut warnings,
            now,
            true,
        )?;

        assert!(warnings.is_empty());
        assert_eq!(
            documents,
            vec![
                HistoryEntry {
                    id: doc_id("doc-1"),
                    timestamp: now - Duration::days(2),
                },
                HistoryEntry {
                    id: doc_id("doc-2"),
                    timestamp: now - Duration::days(1),
                },
                HistoryEntry {
                    id: doc_id("doc-3"),
                    timestamp: now - Duration::days(1),
                },
                HistoryEntry {
                    id: doc_id("doc-4"),
                    timestamp: now - Duration::days(1),
                },
                HistoryEntry {
                    id: doc_id("doc-5"),
                    timestamp: now,
                },
            ],
        );
        Ok(())
    }

    #[test]
    fn test_inconsistent_ordering_warns() -> Result<(), Panic> {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let config = PersonalizationConfig::default();
        let mut warnings = Vec::new();

        validate_history(
            vec![
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-1"),
                    timestamp: Some(now + Duration::days(2)),
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-4"),
                    timestamp: Some(now + Duration::days(1)),
                },
                UnvalidatedHistoryEntry {
                    id: unvalidated_doc_id("doc-5"),
                    timestamp: None,
                },
            ],
            &config,
            &mut warnings,
            now,
            true,
        )?;

        assert_eq!(warnings.len(), 2);
        Ok(())
    }

    #[test]
    fn test_derive_interests_and_tag_weights() -> Result<(), Panic> {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let coi_system = CoiConfig::default().build();
        let (interests, tag_weights) = derive_interests_and_tag_weights(
            &coi_system,
            &vec![
                LoadedHistoryEntry {
                    timestamp: now - Duration::days(4),
                    embedding: Embedding1::from([1., 1.]).normalize()?,
                    tags: vec!["tag-1".try_into()?].try_into()?,
                },
                LoadedHistoryEntry {
                    timestamp: now - Duration::days(3),
                    embedding: Embedding1::from([0., 1.]).normalize()?,
                    tags: DocumentTags::default(),
                },
                LoadedHistoryEntry {
                    timestamp: now - Duration::days(2),
                    embedding: Embedding1::from([0.1, 0.5]).normalize()?,
                    tags: vec!["tag-1".try_into()?, "tag-2".try_into()?].try_into()?,
                },
                LoadedHistoryEntry {
                    timestamp: now - Duration::days(1),
                    embedding: Embedding1::from([1., 0.]).normalize()?,
                    tags: vec!["tag-2".try_into()?, "tag-3".try_into()?].try_into()?,
                },
                LoadedHistoryEntry {
                    timestamp: now,
                    embedding: Embedding1::from([0., 0.]).normalize()?,
                    tags: vec!["tag-3".try_into()?, "tag-1".try_into()?].try_into()?,
                },
            ],
        );

        assert_eq!(
            tag_weights,
            [
                ("tag-1".try_into()?, 3),
                ("tag-2".try_into()?, 2),
                ("tag-3".try_into()?, 2),
            ]
            .into_iter()
            .collect::<HashMap<DocumentTag, usize>>()
        );

        assert!(!interests.is_empty());
        assert_eq!(
            interests
                .iter()
                .fold(0, |acc, coi| acc + coi.stats.view_count),
            5
        );
        assert!(interests.len() <= 5);

        Ok(())
    }

    #[test]
    fn test_history_trimming_trims_new_documents() {
        let now = Utc.with_ymd_and_hms(2000, 10, 20, 3, 4, 5).unwrap();
        let history = vec![
            HistoryEntry {
                id: doc_id("doc-1"),
                timestamp: now - Duration::days(4),
            },
            HistoryEntry {
                id: doc_id("doc-2"),
                timestamp: now - Duration::days(3),
            },
            HistoryEntry {
                id: doc_id("doc-3"),
                timestamp: now - Duration::days(2),
            },
        ];
        let history = trim_history(history, 2);
        assert_eq!(
            history,
            vec![
                HistoryEntry {
                    id: doc_id("doc-2"),
                    timestamp: now - Duration::days(3),
                },
                HistoryEntry {
                    id: doc_id("doc-3"),
                    timestamp: now - Duration::days(2),
                },
            ]
        );
    }
}
