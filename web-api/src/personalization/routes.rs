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
use tracing::instrument;
use xayn_ai_coi::{CoiConfig, CoiSystem};

use super::{
    knn,
    rerank::rerank_by_scores,
    stateless::{
        derive_interests_and_tag_weights,
        load_history,
        trim_history,
        validate_history,
        HistoryEntry,
        UnvalidatedHistoryEntry,
    },
    AppState,
    PersonalizationConfig,
    SemanticSearchConfig,
};
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{BadRequest, DocumentNotFound},
        warning::Warning,
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
    let semantic_search = web::resource("/semantic_search")
        .route(web::post().to(semantic_search.error_with_request_id()));
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
    history: Vec<UnvalidatedHistoryEntry>,
}

impl StatelessPersonalizationRequest {
    /// Return the validated input history, it's expected to be ordered from oldest to newest.
    fn history_and_time(
        self,
        config: &PersonalizationConfig,
        warnings: &mut Vec<Warning>,
    ) -> Result<(Vec<HistoryEntry>, DateTime<Utc>), Error> {
        let history = validate_history(self.history, config, warnings, Utc::now(), false)?;
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
    let (history, time) = request.history_and_time(state.config.as_ref(), &mut warnings)?;

    let excluded = history.iter().map(|entry| entry.id.clone()).collect_vec();
    let history = trim_history(
        history,
        state.config.personalization.max_stateless_history_for_cois,
    );
    let history = load_history(&state.storage, history).await?;
    let (interests, tag_weights) = derive_interests_and_tag_weights(&state.coi, &history);

    let mut documents = knn::CoiSearch {
        interests: &interests.positive,
        excluded: &excluded,
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

#[derive(Default, Deserialize)]
#[serde(default)]
struct UnvalidatedInputUser {
    id: Option<String>,
    history: Option<Vec<UnvalidatedHistoryEntry>>,
}

enum InputUser {
    Ref { id: UserId },
    Inline { history: Vec<HistoryEntry> },
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

#[derive(Deserialize)]
struct UnvalidatedSemanticSearchQuery {
    document_id: String,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    min_similarity: Option<f32>,
    #[serde(default)]
    personalize: Option<UnvalidatedPersonalize>,
}

impl UnvalidatedSemanticSearchQuery {
    fn validate_and_resolve_defaults(
        self,
        config: &(impl AsRef<SemanticSearchConfig> + AsRef<PersonalizationConfig>),
        warnings: &mut Vec<Warning>,
    ) -> Result<SemanticSearchQuery, Error> {
        let semantic_search_config: &SemanticSearchConfig = config.as_ref();
        Ok(SemanticSearchQuery {
            document_id: self.document_id.try_into()?,
            count: validate_return_count(
                self.count,
                semantic_search_config.max_number_documents,
                semantic_search_config.default_number_documents,
            )?,
            min_similarity: self.min_similarity.map(|value| value.clamp(0., 1.)),
            personalize: self
                .personalize
                .map(|personalize| personalize.validate(config.as_ref(), warnings))
                .transpose()?,
        })
    }
}
#[derive(Deserialize)]
struct UnvalidatedPersonalize {
    #[serde(default = "true_fn")]
    exclude_seen: bool,
    user: UnvalidatedInputUser,
}

fn true_fn() -> bool {
    true
}

impl UnvalidatedPersonalize {
    fn validate(
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

struct SemanticSearchQuery {
    document_id: DocumentId,
    count: usize,
    min_similarity: Option<f32>,
    personalize: Option<Personalize>,
}

struct Personalize {
    exclude_seen: bool,
    user: InputUser,
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
    Json(query): Json<UnvalidatedSemanticSearchQuery>,
) -> Result<impl Responder, Error> {
    let mut warnings = Vec::new();

    let SemanticSearchQuery {
        document_id,
        count,
        min_similarity,
        personalize,
    } = query.validate_and_resolve_defaults(&state.config, &mut warnings)?;

    let embedding = storage::Document::get_embedding(&state.storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    let mut excluded = if let Some(personalize) = &personalize {
        personalized_exclusions(&state.storage, state.config.as_ref(), personalize).await?
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

    if let Some(personalize) = personalize {
        personalize_knn_search_result(
            &state.storage,
            &state.config,
            &state.coi,
            personalize,
            &mut documents,
        )
        .await?;
    }

    Ok(Json(SemanticSearchResponse {
        documents: documents.into_iter().map_into().collect(),
    }))
}

async fn personalized_exclusions(
    storage: &impl storage::Interaction,
    config: &PersonalizationConfig,
    personalize: &Personalize,
) -> Result<Vec<DocumentId>, Error> {
    if !personalize.exclude_seen {
        return Ok(Vec::new());
    }

    Ok(match &personalize.user {
        InputUser::Ref { id } => {
            //FIXME move optimization into storage abstraction
            if config.store_user_history {
                storage::Interaction::get(storage, id).await?
            } else {
                Vec::new()
            }
        }
        InputUser::Inline { history } => history.iter().map(|entry| entry.id.clone()).collect(),
    })
}

async fn personalize_knn_search_result(
    storage: &(impl storage::Interest + storage::Tag + storage::Document),
    config: &(impl AsRef<CoiConfig> + AsRef<SemanticSearchConfig> + AsRef<PersonalizationConfig>),
    coi_system: &CoiSystem,
    personalize: Personalize,
    documents: &mut [PersonalizedDocument],
) -> Result<(), Error> {
    let (interests, tag_weights) = match personalize.user {
        InputUser::Ref { id } => (
            storage::Interest::get(storage, &id).await?,
            storage::Tag::get(storage, &id).await?,
        ),
        InputUser::Inline { history } => {
            let config: &PersonalizationConfig = config.as_ref();
            let history = trim_history(history, config.max_stateless_history_for_cois);
            let history = load_history(storage, history).await?;
            derive_interests_and_tag_weights(coi_system, &history)
        }
    };

    if interests.has_enough(config.as_ref()) {
        let config: &SemanticSearchConfig = config.as_ref();
        rerank_by_scores(
            coi_system,
            documents,
            &interests,
            &tag_weights,
            config.score_weights,
            Utc::now(),
        );
    }
    Ok(())
}
