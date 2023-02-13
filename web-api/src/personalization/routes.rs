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
use xayn_ai_coi::CoiSystem;

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
        common::{BadRequest, DocumentNotFound},
    },
    models::{DocumentId, DocumentProperties, PersonalizedDocument, UserId, UserInteractionType},
    storage::{self, KnnSearchParams},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
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

    config.service(users).service(semantic_search);
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
            count: options.document_count(&state.config.personalization)?,
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
