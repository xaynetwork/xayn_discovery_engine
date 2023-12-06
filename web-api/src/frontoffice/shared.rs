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

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use xayn_ai_coi::CoiSystem;

use super::{
    stateless::{validate_history, HistoryEntry, UnvalidatedHistoryEntry},
    PersonalizationConfig,
};
use crate::{
    error::{
        common::{BadRequest, InvalidDocumentCount},
        warning::Warning,
    },
    models::{SnippetId, SnippetOrDocumentId, UserId},
    storage::{self, Exclusions},
    Error,
};
#[cfg(test)]
use crate::{
    frontoffice::filter::Filter,
    frontoffice::knn,
    frontoffice::rerank::rerank,
    models::DocumentId,
    models::PersonalizedDocument,
};

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub(crate) enum PersonalizedDocumentsError {
    NotEnoughInteractions,
}
#[cfg(test)]
pub(crate) enum PersonalizeBy<'a> {
    KnnSearch {
        count: usize,
        filter: Option<&'a Filter>,
    },
    #[cfg(test)]
    Documents(&'a [&'a DocumentId]),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedInputUser {
    id: Option<String>,
    history: Option<Vec<UnvalidatedHistoryEntry>>,
}

impl UnvalidatedInputUser {
    fn validate(
        self,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<InputUser, Error> {
        Ok(match (self.id, self.history) {
            (Some(id), None) => InputUser::Ref { id: id.try_into()? },
            (None, Some(history)) => InputUser::Inline {
                history: validate_history(history, config, warnings, Utc::now(), true)?,
            },
            _ => {
                return Err(BadRequest::from(
                    "personalize.user must have _either_ an `id` or a `history` field",
                )
                .into())
            }
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub(crate) enum UnvalidatedSnippetOrDocumentId {
    DocumentId(String),
    SnippetId { document_id: String, sub_id: u32 },
}

impl UnvalidatedSnippetOrDocumentId {
    pub(super) fn validate(self) -> Result<SnippetOrDocumentId, Error> {
        Ok(match self {
            UnvalidatedSnippetOrDocumentId::DocumentId(document_id) => {
                SnippetOrDocumentId::DocumentId(document_id.try_into()?)
            }
            UnvalidatedSnippetOrDocumentId::SnippetId {
                document_id,
                sub_id,
            } => SnippetOrDocumentId::SnippetId(SnippetId::new(document_id.try_into()?, sub_id)),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UnvalidatedPersonalize {
    #[serde(default = "default_exclude_seen")]
    exclude_seen: bool,
    user: UnvalidatedInputUser,
}

impl UnvalidatedPersonalize {
    pub(super) fn validate(
        self,
        personalization_config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<Personalize, Error> {
        Ok(Personalize {
            exclude_seen: self.exclude_seen,
            user: self.user.validate(personalization_config, warnings)?,
        })
    }
}

pub(super) enum InputUser {
    Ref { id: UserId },
    Inline { history: Vec<HistoryEntry> },
}

pub(super) struct Personalize {
    pub(crate) exclude_seen: bool,
    pub(crate) user: InputUser,
}

pub(super) const fn default_exclude_seen() -> bool {
    true
}

pub(super) const fn default_include_properties() -> bool {
    true
}

pub(super) fn validate_count(
    count: usize,
    max: usize,
    candidates: usize,
) -> Result<(), InvalidDocumentCount> {
    let min = 1;
    let max = max.min(candidates);
    if !(min..=max).contains(&count) {
        return Err(InvalidDocumentCount { count, min, max });
    }

    Ok(())
}

pub(super) async fn personalized_exclusions(
    storage: &impl storage::Interaction,
    config: &PersonalizationConfig,
    personalize: &Personalize,
) -> Result<Exclusions, Error> {
    if !personalize.exclude_seen {
        return Ok(Exclusions::default());
    }

    Ok(match &personalize.user {
        InputUser::Ref { id } => {
            //FIXME move optimization into storage abstraction
            if config.store_user_history {
                let documents = storage::Interaction::get(storage, id).await?;
                Exclusions {
                    documents,
                    snippets: Vec::new(),
                }
            } else {
                Exclusions::default()
            }
        }
        InputUser::Inline { history } => {
            let (documents, snippets) =
                history
                    .iter()
                    .partition_map(|entry| match entry.id.clone() {
                        SnippetOrDocumentId::SnippetId(id) => either::Either::Right(id),
                        SnippetOrDocumentId::DocumentId(id) => either::Either::Left(id),
                    });
            Exclusions {
                documents,
                snippets,
            }
        }
    })
}

pub(crate) async fn update_interactions(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    interactions: Vec<SnippetOrDocumentId>,
    store_user_history: bool,
    time: DateTime<Utc>,
) -> Result<(), Error> {
    storage::Interaction::user_seen(storage, user_id, time).await?;

    storage::Interaction::update_interactions(
        storage,
        user_id,
        interactions,
        store_user_history,
        time,
        |context| {
            for tag in &context.document.tags {
                *context.tag_weight_diff
                    .get_mut(tag)
                    .unwrap(/* update_interactions assures all tags are given */) += 1;
            }
            coi.log_user_reaction(context.interests, &context.document.embedding, context.time)
                .clone()
        },
    )
    .await?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn personalize_documents_by(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi_system: &CoiSystem,
    user_id: &UserId,
    personalization: &PersonalizationConfig,
    by: PersonalizeBy<'_>,
    time: DateTime<Utc>,
    include_properties: bool,
    include_snippet: bool,
) -> Result<Option<Vec<PersonalizedDocument>>, Error> {
    storage::Interaction::user_seen(storage, user_id, time).await?;

    let interests = storage::Interest::get(storage, user_id).await?;

    if interests.len() < coi_system.config().min_cois() {
        return Ok(None);
    }

    let excluded = if personalization.store_user_history {
        Exclusions {
            documents: storage::Interaction::get(storage, user_id).await?,
            snippets: Vec::new(),
        }
    } else {
        Exclusions::default()
    };

    let mut documents = match by {
        PersonalizeBy::KnnSearch { count, filter } => {
            knn::CoiSearch {
                interests: &interests,
                excluded: &excluded,
                horizon: coi_system.config().horizon(),
                max_cois: personalization.max_cois_for_knn,
                count,
                num_candidates: personalization.max_number_candidates,
                time,
                include_properties,
                include_snippet,
                filter,
            }
            .run_on(storage)
            .await?
        }

        PersonalizeBy::Documents(documents) => {
            let ids = documents
                .iter()
                .map(|&id| SnippetId::new(id.clone(), 0))
                .collect_vec();
            storage::Document::get_personalized(
                storage,
                ids.iter(),
                include_properties,
                include_snippet,
            )
            .await?
        }
    };

    let tag_weights = storage::Tag::get(storage, user_id).await?;

    rerank(
        coi_system,
        &mut documents,
        &interests,
        &tag_weights,
        personalization.score_weights,
        time,
    );

    #[cfg_attr(not(test), allow(irrefutable_let_patterns))]
    if let PersonalizeBy::KnnSearch { count, .. } = by {
        // due to ceiling the number of documents we fetch per COI
        // we might end up with more documents than we want
        documents.truncate(count);
    }

    Ok(Some(documents))
}
