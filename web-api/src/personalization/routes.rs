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
use regex::internal::Input;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use xayn_ai_coi::{CoiSystem, UserInterests};

use super::{
    knn,
    rerank::rerank_by_scores,
    stateless::{derive_interests_and_tag_weights, HistoryEntry, InvalidatedHistory},
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
    let stateless = web::resource("personalized_documents")
        .route(web::post().to(stateless_personalized_documents.error_with_request_id()));

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
    interactions: impl IntoIterator<
        IntoIter = impl Clone + ExactSizeIterator<Item = &(DocumentId, UserInteractionType)>,
    >,
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
            match context.interaction_type {
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
    history: InvalidatedHistory,
}

impl StatelessPersonalizationRequest {
    async fn history_and_time(
        self,
        storage: &impl storage::Document,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<(Vec<HistoryEntry>, DateTime<Utc>), Error> {
        let history = self
            .history
            .validate_and_load(storage, config, warnings)
            .await?;
        let time = history.last().unwrap(/* history is checked to be not empty */).timestamp;
        Ok((history, time))
    }

    fn document_count(&self, config: &PersonalizationConfig) -> Result<usize, Error> {
        validate_return_count(
            self.count,
            config.max_number_documents,
            config.default_number_documents,
        )
    }
}

#[instrument(skip_all)]
async fn stateless_personalized_documents(
    state: Data<AppState>,
    Json(request): Json<StatelessPersonalizationRequest>,
) -> Result<impl Responder, Error> {
    let mut warnings = vec![];
    let published_after = request.published_after;
    let count = request.document_count(state.config.as_ref())?;
    let (history, time) = request
        .history_and_time(&state.storage, state.config.as_ref(), &mut warnings)
        .await?;

    let (interests, tag_weights) = derive_interests_and_tag_weights(&state.coi, &history);

    let mut documents = knn::CoiSearch {
        interests: &interests.positive,
        excluded: history.iter().map(|entry| &entry.id),
        horizon: state.coi.config().horizon(),
        max_cois: state.config.personalization.max_cois_for_knn,
        count,
        published_after,
        time,
    }
    .run_on(&state.storage)
    .await?;

    rerank_by_scores(
        &state.coi,
        &mut documents,
        &interests,
        &tag_weights,
        state.config.personalization.score_weights,
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
        validate_return_count(
            self.count,
            config.max_number_documents,
            config.default_number_documents,
        )
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
    #[cfg_attr(not(test), allow(dead_code))]
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
            knn::CoiSearch {
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
            storage::Document::get_personalized(storage, documents.iter().copied()).await?
        }
    };

    let tag_weights = storage::Tag::get(storage, user_id).await?;

    rerank_by_scores(
        coi_system,
        &mut documents,
        &interests,
        &tag_weights,
        personalization.score_weights,
        time,
    );

    if let PersonalizeBy::KnnSearch { count, .. } = by {
        // due to ceil-ing the number of documents we fetch per COI
        // we might end up with more documents then we want
        documents.truncate(count);
    }

    Ok(Some(documents))
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum InvalidatedInputUser {
    Ref(String),
    Inline { history: Vec<String> },
}

enum InputUser {
    Ref(UserId),
    Inline { history: Vec<DocumentId> },
}

impl TryFrom<InvalidatedInputUser> for InputUser {
    type Error = Error;

    fn try_from(user: InvalidatedInputUser) -> Result<Self, Self::Error> {
        Ok(match user {
            InvalidatedInputUser::Ref(id) => Self::Ref(id.try_into()?),
            InvalidatedInputUser::Inline { history } => Self::Inline {
                history: history
                    .into_iter()
                    .map(DocumentId::try_from)
                    .try_collect()?,
            },
        })
    }
}

#[derive(Deserialize)]
struct InvalidatedSemanticSearchQuery {
    document_id: String,
    count: Option<usize>,
    min_similarity: Option<f32>,
    user: Option<InvalidatedInputUser>,
}

struct SemanticSearchQuery {
    document_id: DocumentId,
    count: usize,
    min_similarity: Option<f32>,
    user: Option<InputUser>,
}

impl InvalidatedSemanticSearchQuery {
    fn validate_and_resolve_defaults(
        self,
        config: &SemanticSearchConfig,
    ) -> Result<SemanticSearchQuery, Error> {
        Ok(SemanticSearchQuery {
            document_id: self.document_id.try_into()?,
            count: validate_return_count(
                self.count,
                config.max_number_documents,
                config.default_number_documents,
            )?,
            min_similarity: self.min_similarity.map(|value| value.clamp(0., 1.)),
            user: self.user.map(InputUser::try_from).transpose()?,
        })
    }
}

fn validate_return_count(
    input: Option<usize>,
    max_value: usize,
    default_value: usize,
) -> Result<usize, Error> {
    let count = input.map_or(default_value, |count| count.min(max_value));

    if count > 0 {
        Ok(count)
    } else {
        Err(BadRequest::from("count has to be at least 1").into())
    }
}

#[derive(Serialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

async fn semantic_search(
    state: Data<AppState>,
    query: Json<InvalidatedSemanticSearchQuery>,
) -> Result<impl Responder, Error> {
    let SemanticSearchQuery {
        document_id,
        count,
        min_similarity,
        user,
    } = query
        .into_inner()
        .validate_and_resolve_defaults(state.config.as_ref())?;

    let embedding = storage::Document::get_embedding(&state.storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    let mut excluded = if let Some(user) = &user {
        match user {
            InputUser::Ref(user_id) => {
                //FIXME move optimization into storage abstraction
                if state.config.personalization.store_user_history {
                    storage::Interaction::get(&state.storage, user_id).await?
                } else {
                    Vec::with_capacity(1)
                }
            }
            InputUser::Inline { history } => history.clone(),
        }
    } else {
        Vec::with_capacity(1)
    };
    excluded.push(document_id);

    let mut documents = storage::Document::get_by_embedding(
        &state.storage,
        KnnSearchParams {
            excluded: &excluded,
            embedding: &embedding,
            k_neighbors: count,
            num_candidates: count,
            published_after: None,
            min_similarity,
            time: Utc::now(),
        },
    )
    .await?;

    if let Some(user) = &user {
        let reranking_data = match user {
            InputUser::Ref(id) => {
                fetch_interests_and_tag_weights(id, &state.storage, &state.config).await?
            }
            InputUser::Inline { history } => {
                derive_interests_and_tag_weights(history, state.config.as_ref())?
            }
        };

        if let Some((interests, tag_weights)) = reranking_data {
            rerank_by_scores(
                &state.coi,
                &mut documents,
                &interests,
                &tag_weights,
                state.config.semantic_search.score_weights,
                Utc::now(),
            );
        }
    }

    Ok(Json(SemanticSearchResponse {
        documents: documents.into_iter().map_into().collect(),
    }))
}

async fn fetch_interests_and_tag_weights(
    user_id: &UserId,
    storage: &(impl storage::Interest + storage::Tag),
    config: &impl AsRef<xayn_ai_coi::CoiConfig>,
) -> Result<Option<(UserInterests, TagWeights)>, Error> {
    let interests = storage::Interest::get(storage, user_id).await?;
    if interests.has_enough(config.as_ref()) {
        let tag_weights = storage::Tag::get(storage, user_id).await?;
        Ok(Some((interests, tag_weights)))
    } else {
        Ok(None)
    }
}

type TagWeights = HashMap<DocumentTag, usize>;
