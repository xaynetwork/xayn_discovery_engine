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

use std::collections::HashMap;

use actix_web::{
    http::StatusCode,
    web::{self, Data, Json, Path, Query, ServiceConfig},
    Either,
    HttpResponse,
    Responder,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use xayn_ai_coi::{CoiSystem, UserInterests};

use super::{
    knn,
    rerank::rerank_by_interest_and_tag_weight,
    AppState,
    PersonalizationConfig,
    SemanticSearchConfig,
};
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{BadRequest, DocumentNotFound, HistoryTooSmall},
        warning::Warning,
    },
    models::{
        DocumentId,
        DocumentProperties,
        DocumentTag,
        InteractedDocument,
        PersonalizedDocument,
        UserId,
        UserInteractionType,
    },
    storage::{self, KnnSearchParams},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let stateless = web::resource("personalized_documents")
        .route(web::post().to(stateless_personalized_documents.error_with_request_id()));

    let users = web::scope("/users/{user_id}")
        .service(
            web::resource("interactions")
                .route(web::patch().to(interactions.error_with_request_id())),
        )
        .service(
            web::resource("personalized_documents")
                .route(web::get().to(personalized_documents.error_with_request_id())),
        );

    let semantic_search = web::resource("/semantic_search/{document_id}")
        .route(web::get().to(semantic_search.error_with_request_id()));

    config
        .service(users)
        .service(semantic_search)
        .service(stateless);
}

/// Represents user interaction request body.
#[derive(Clone, Debug, Deserialize)]
struct UpdateInteractions {
    documents: Vec<UserInteractionData>,
}

#[derive(Clone, Debug, Deserialize)]
struct UserInteractionData {
    #[serde(rename = "id")]
    pub(crate) document_id: String,
    #[serde(rename = "type")]
    pub(crate) interaction_type: UserInteractionType,
}

async fn interactions(
    state: Data<AppState>,
    user_id: Path<String>,
    Json(interactions): Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let interactions = interactions
        .documents
        .into_iter()
        .map(|data| {
            data.document_id
                .try_into()
                .map(|document_id| (document_id, data.interaction_type))
        })
        .try_collect::<_, Vec<_>, _>()?;
    update_interactions(
        &state.storage,
        &state.coi,
        &user_id,
        &interactions,
        state.config.personalization.store_user_history,
        Utc::now(),
    )
    .await?;

    Ok(HttpResponse::NoContent())
}

pub(crate) async fn update_interactions(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    interactions: &[(DocumentId, UserInteractionType)],
    store_user_history: bool,
    time: DateTime<Utc>,
) -> Result<(), Error> {
    storage::Interaction::user_seen(storage, user_id, time).await?;

    #[allow(clippy::zero_sized_map_values)]
    let document_id_to_interaction_type = interactions
        .iter()
        .map(|(document_id, interaction_type)| (document_id, interaction_type))
        .collect::<HashMap<_, _>>();

    let document_ids = interactions
        .iter()
        .map(|(document_id, _)| document_id)
        .collect_vec();
    storage::Interaction::update_interactions(
        storage,
        user_id,
        &document_ids,
        store_user_history,
        time,
        |context| {
            match document_id_to_interaction_type[&context.document.id] {
                UserInteractionType::Positive => {
                    for tag in &context.document.tags {
                        *context.tag_weight_diff
                            .get_mut(tag)
                            .unwrap(/* update_interactions assures all tags are given */) += 1;
                    }
                    coi.log_positive_user_reaction(
                        context.positive_cois,
                        &context.document.embedding,
                        context.time,
                    )
                    .clone()
                }
            }
        },
    )
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct StatelessPersonalizationRequest {
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    published_after: Option<DateTime<Utc>>,
    history: Vec<UncheckedHistoryEntry>,
}

impl StatelessPersonalizationRequest {
    fn history(
        self,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<Vec<HistoryEntry>, Error> {
        if self.history.is_empty() {
            return Err(HistoryTooSmall.into());
        }

        let max_history_len = config.max_stateless_history_size;
        if self.history.len() > max_history_len {
            warnings.push(format!("history truncated, max length is {max_history_len}").into());
        }
        let mut most_recent_time = Utc::now();
        //input is from oldest to newest
        let mut history = self
            .history
            .into_iter()
            .rev()
            .take(max_history_len)
            .map(|unchecked| -> Result<_, Error> {
                let id = unchecked.id.try_into()?;
                let timestamp = unchecked.timestamp.unwrap_or(most_recent_time);
                if timestamp > most_recent_time {
                    warnings
                        .push(format!("inconsistent history ordering around document {id}").into());
                }
                most_recent_time = timestamp;
                Ok(HistoryEntry { id, timestamp })
            })
            .try_collect::<_, Vec<_>, _>()?;
        history.reverse();
        Ok(history)
    }

    fn document_count(&self, config: &PersonalizationConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }
}

#[derive(Deserialize)]
struct UncheckedHistoryEntry {
    id: String,
    timestamp: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct HistoryEntry {
    id: DocumentId,
    timestamp: DateTime<Utc>,
}

#[instrument(skip_all)]
async fn stateless_personalized_documents(
    state: Data<AppState>,
    Json(request): Json<StatelessPersonalizationRequest>,
) -> Result<impl Responder, Error> {
    let mut warnings = vec![];
    let published_after = request.published_after;
    let count = request.document_count(state.config.as_ref())?;
    let history = request.history(state.config.as_ref(), &mut warnings)?;
    let ids = history.iter().map(|document| &document.id).collect_vec();
    let time = history
        .last()
        .unwrap(/* history has been checked */)
        .timestamp;

    let documents_from_history = storage::Document::get_interacted(&state.storage, &ids).await?;
    let documents_from_history = documents_from_history
        .iter()
        .map(|document| (&document.id, document))
        .collect::<HashMap<_, _>>();

    let mut interests = UserInterests::default();
    for entry in &history {
        let id = &entry.id;
        if let Some(document) = documents_from_history.get(id) {
            state.coi.log_positive_user_reaction(
                &mut interests.positive,
                &document.embedding,
                entry.timestamp,
            );
        } else {
            let msg = format!("document {id} does not exist");
            warn!("{}", msg);
            warnings.push(msg.into());
        }
    }

    let mut documents = personalized_knn::Search {
        interests: &interests.positive,
        excluded: &[],
        horizon: state.coi.config().horizon(),
        max_cois: state.config.personalization.max_cois_for_knn,
        count,
        published_after,
        time,
    }
    .run_on(&state.storage)
    .await?;

    let tag_weights = tag_weights_from_history(documents_from_history.values().copied());

    rerank_by_interest_and_tag_weight(
        &state.coi,
        &mut documents,
        &interests,
        &tag_weights,
        state.config.personalization.interest_tag_bias,
        time,
    );

    Ok(Json(StatelessPersonalizationResponse {
        documents: documents.into_iter().map(Into::into).collect(),
        warnings,
    }))
}

fn tag_weights_from_history<'a>(
    documents_in_history: impl IntoIterator<Item = &'a InteractedDocument>,
) -> HashMap<DocumentTag, usize> {
    let mut weights = HashMap::default();
    for document in documents_in_history {
        for tag in &document.tags {
            *weights.entry(tag.clone()).or_default() += 1;
        }
    }
    weights
}

#[derive(Serialize)]
struct StatelessPersonalizationResponse {
    documents: Vec<PersonalizedDocumentData>,
    warnings: Vec<Warning>,
}
/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    pub(crate) count: Option<usize>,
    pub(crate) published_after: Option<DateTime<Utc>>,
}

impl PersonalizedDocumentsQuery {
    fn document_count(&self, config: &PersonalizationConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }
}

async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<String>,
    options: Query<PersonalizedDocumentsQuery>,
) -> Result<impl Responder, Error> {
    personalize_documents_by(
        &state.storage,
        &state.coi,
        &user_id.into_inner().try_into()?,
        &state.config.personalization,
        PersonalizeBy::KnnSearch {
            count: options.document_count(state.config.as_ref())?,
            published_after: options.published_after,
        },
        Utc::now(),
    )
    .await
    .map(|documents| {
        if let Some(documents) = documents {
            Either::Left(Json(PersonalizedDocumentsResponse {
                documents: documents.into_iter().map_into().collect(),
            }))
        } else {
            Either::Right((
                Json(PersonalizedDocumentsError::NotEnoughInteractions),
                StatusCode::CONFLICT,
            ))
        }
    })
}

#[derive(Debug, Serialize)]
struct PersonalizedDocumentData {
    id: DocumentId,
    score: f32,
    #[serde(skip_serializing_if = "DocumentProperties::is_empty")]
    properties: DocumentProperties,
}

impl From<PersonalizedDocument> for PersonalizedDocumentData {
    fn from(document: PersonalizedDocument) -> Self {
        Self {
            id: document.id,
            score: document.score,
            properties: document.properties,
        }
    }
}

/// Represents response from personalized documents endpoint.
#[derive(Debug, Serialize)]
struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    documents: Vec<PersonalizedDocumentData>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub(crate) enum PersonalizedDocumentsError {
    NotEnoughInteractions,
}

pub(crate) enum PersonalizeBy<'a> {
    KnnSearch {
        count: usize,
        published_after: Option<DateTime<Utc>>,
    },
    #[allow(dead_code)]
    Documents(&'a [&'a DocumentId]),
}

pub(crate) async fn personalize_documents_by(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi_system: &CoiSystem,
    user_id: &UserId,
    personalization: &PersonalizationConfig,
    by: PersonalizeBy<'_>,
    time: DateTime<Utc>,
) -> Result<Option<Vec<PersonalizedDocument>>, Error> {
    storage::Interaction::user_seen(storage, user_id, time).await?;

    let interests = storage::Interest::get(storage, user_id).await?;

    if !interests.has_enough(coi_system.config()) {
        return Ok(None);
    }

    let excluded = if personalization.store_user_history {
        storage::Interaction::get(storage, user_id).await?
    } else {
        Vec::new()
    };

    let mut documents = match by {
        PersonalizeBy::KnnSearch {
            count,
            published_after,
        } => {
            knn::Search {
                interests: &interests.positive,
                excluded: &excluded,
                horizon: coi_system.config().horizon(),
                max_cois: personalization.max_cois_for_knn,
                count,
                published_after,
                time,
            }
            .run_on(storage)
            .await?
        }
        PersonalizeBy::Documents(documents) => {
            storage::Document::get_personalized(storage, documents).await?
        }
    };

    let tag_weights = storage::Tag::get(storage, user_id).await?;

    rerank_by_interest_and_tag_weight(
        coi_system,
        &mut documents,
        &interests,
        &tag_weights,
        personalization.interest_tag_bias,
        time,
    );

    if let PersonalizeBy::KnnSearch { count, .. } = by {
        // due to ceil-ing the number of documents we fetch per COI
        // we might end up with more documents then we want
        documents.truncate(count);
    }

    Ok(Some(documents))
}

#[derive(Deserialize)]
struct SemanticSearchQuery {
    count: Option<usize>,
    min_similarity: Option<f32>,
}

impl SemanticSearchQuery {
    fn document_count(&self, config: &SemanticSearchConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }

    fn min_similarity(&self) -> Option<f32> {
        self.min_similarity.map(|value| value.clamp(0., 1.))
    }
}

#[derive(Serialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

async fn semantic_search(
    state: Data<AppState>,
    document_id: Path<String>,
    query: Query<SemanticSearchQuery>,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let count = query.document_count(state.config.as_ref())?;
    let min_similarity = query.min_similarity();

    let embedding = storage::Document::get_embedding(&state.storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    let documents = storage::Document::get_by_embedding(
        &state.storage,
        KnnSearchParams {
            excluded: &[document_id],
            embedding: &embedding,
            k_neighbors: count,
            num_candidates: count,
            published_after: None,
            min_similarity,
            time: Utc::now(),
        },
    )
    .await?;

    Ok(Json(SemanticSearchResponse {
        documents: documents.into_iter().map_into().collect(),
    }))
}
