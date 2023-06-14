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
    app::TenantState,
    error::{
        common::{BadRequest, DocumentNotFound},
        warning::Warning,
    },
    models::{DocumentId, DocumentProperties, PersonalizedDocument, UserId},
    storage::{self, KnnSearchParams, MergeFn, NormalizationFn, SearchStrategy},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let users = web::scope("/users/{user_id}")
        .service(web::resource("interactions").route(web::patch().to(interactions)))
        .service(
            web::resource("personalized_documents").route(web::get().to(personalized_documents)),
        );
    let semantic_search = web::resource("/semantic_search").route(web::post().to(semantic_search));

    config.service(users).service(semantic_search);
}

/// Represents user interaction request body.
#[derive(Debug, Deserialize)]
struct UpdateInteractions {
    documents: Vec<UserInteractionData>,
}

#[derive(Debug, Deserialize)]
struct UserInteractionData {
    #[serde(rename = "id")]
    document_id: String,
}

async fn interactions(
    state: Data<AppState>,
    user_id: Path<String>,
    Json(interactions): Json<UpdateInteractions>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let interactions = interactions
        .documents
        .into_iter()
        .map(|data| data.document_id.try_into())
        .try_collect::<_, Vec<_>, _>()?;
    update_interactions(
        &storage,
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
    interactions: impl IntoIterator<IntoIter = impl Clone + ExactSizeIterator<Item = &DocumentId>>,
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

/// Represents personalized documents query params.
#[derive(Debug, Deserialize)]
struct PersonalizedDocumentsQuery {
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
}

impl PersonalizedDocumentsQuery {
    fn to_personalize_by_knn(
        &self,
        config: &PersonalizationConfig,
    ) -> Result<PersonalizeBy<'_>, Error> {
        let count = validate_return_count(
            self.count,
            config.max_number_documents,
            config.default_number_documents,
        )?;
        let published_after = self.published_after;

        Ok(PersonalizeBy::KnnSearch {
            count,
            published_after,
        })
    }
}

async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<String>,
    params: Query<PersonalizedDocumentsQuery>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    personalize_documents_by(
        &storage,
        &state.coi,
        &user_id.into_inner().try_into()?,
        &state.config.personalization,
        params.to_personalize_by_knn(state.config.as_ref())?,
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
enum PersonalizedDocumentsError {
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

    if interests.len() < coi_system.config().min_cois() {
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
                interests: &interests,
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
        // due to ceiling the number of documents we fetch per COI
        // we might end up with more documents than we want
        documents.truncate(count);
    }

    Ok(Some(documents))
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
struct UnvalidatedSemanticSearchQuery {
    document: UnvalidatedInputDocument,
    count: Option<usize>,
    min_similarity: Option<f32>,
    published_after: Option<DateTime<Utc>>,
    personalize: Option<UnvalidatedPersonalize>,
    #[serde(default)]
    enable_hybrid_search: bool,
    // TODO[pmk/now] to we keep enable_hybrid_search
    #[serde(default, rename = "_dev")]
    dev: Option<DevOptions>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DevOptions {
    hybrid: Option<HybridDevOption>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HybridDevOption {
    EsRrf {
        rank_constant: Option<u32>,
    },
    Customize {
        normalize_knn: NormalizationFn,
        normalize_bm25: NormalizationFn,
        merge_fn: MergeFn,
    },
}

fn create_search_strategy<'a>(
    enable_hybrid_search: bool,
    dev: Option<&DevOptions>,
    query: Option<&'a str>,
) -> SearchStrategy<'a> {
    if !enable_hybrid_search {
        return SearchStrategy::Knn;
    }
    let Some(query) = query else {
        return SearchStrategy::Knn;
    };
    let Some(DevOptions { hybrid: Some(hybrid) }) = dev else {
        return SearchStrategy::Hybrid { query };
    };

    match hybrid.clone() {
        HybridDevOption::EsRrf { rank_constant } => SearchStrategy::HybridEsRrf {
            query,
            rank_constant,
        },
        HybridDevOption::Customize {
            normalize_knn,
            normalize_bm25,
            merge_fn,
        } => SearchStrategy::HybridDev {
            query,
            normalize_knn,
            normalize_bm25,
            merge_fn,
        },
    }
}

impl UnvalidatedSemanticSearchQuery {
    fn validate_and_resolve_defaults(
        self,
        config: &(impl AsRef<SemanticSearchConfig> + AsRef<PersonalizationConfig>),
        warnings: &mut Vec<Warning>,
    ) -> Result<SemanticSearchQuery, Error> {
        let Self {
            document,
            count,
            min_similarity,
            published_after,
            personalize,
            enable_hybrid_search,
            dev,
        } = self;
        let semantic_search_config: &SemanticSearchConfig = config.as_ref();

        Ok(SemanticSearchQuery {
            document: document.validate()?,
            published_after,
            count: validate_return_count(
                count,
                semantic_search_config.max_number_documents,
                semantic_search_config.default_number_documents,
            )?,
            min_similarity: min_similarity.map(|value| value.clamp(0., 1.)),
            personalize: personalize
                .map(|personalize| personalize.validate(config.as_ref(), warnings))
                .transpose()?,
            enable_hybrid_search,
            dev,
        })
    }
}

#[derive(Debug, Deserialize)]
struct UnvalidatedInputDocument {
    id: Option<String>,
    query: Option<String>,
}

impl UnvalidatedInputDocument {
    fn validate(self) -> Result<InputDocument, Error> {
        match (self.id, self.query) {
            (Some(_), Some(_)) => Err(BadRequest::from(
                "either id or query must be present in the request, but both were found",
            )
            .into()),
            (None, Some(query)) => Ok(InputDocument::Query(query)),
            (Some(id), None) => Ok(InputDocument::Ref(id.try_into()?)),
            (None, None) => {
                Err(BadRequest::from("either id or query must be present in the request").into())
            }
        }
    }
}

#[derive(Debug, Deserialize)]
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
    document: InputDocument,
    count: usize,
    min_similarity: Option<f32>,
    published_after: Option<DateTime<Utc>>,
    personalize: Option<Personalize>,
    enable_hybrid_search: bool,
    dev: Option<DevOptions>,
}

enum InputDocument {
    Ref(DocumentId),
    Query(String),
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
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let mut warnings = Vec::new();

    let SemanticSearchQuery {
        document,
        count,
        min_similarity,
        personalize,
        published_after,
        enable_hybrid_search,
        dev,
    } = query.validate_and_resolve_defaults(&state.config, &mut warnings)?;

    let mut excluded = if let Some(personalize) = &personalize {
        personalized_exclusions(&storage, state.config.as_ref(), personalize).await?
    } else {
        Vec::new()
    };
    let query: String;
    let (embedding, strategy) = match document {
        InputDocument::Ref(id) => {
            let embedding = storage::Document::get_embedding(&storage, &id)
                .await?
                .ok_or(DocumentNotFound)?;
            excluded.push(id);
            (
                embedding,
                create_search_strategy(enable_hybrid_search, dev.as_ref(), None),
            )
        }
        InputDocument::Query(query_) => {
            query = query_;
            let embedding = state.embedder.run(&query)?;
            (
                embedding,
                create_search_strategy(enable_hybrid_search, dev.as_ref(), Some(&query)),
            )
        }
    };

    let mut documents = storage::Document::get_by_embedding(
        &storage,
        KnnSearchParams {
            excluded: &excluded,
            embedding: &embedding,
            count,
            num_candidates: count,
            published_after,
            min_similarity,
            strategy,
        },
    )
    .await?;

    if let Some(personalize) = personalize {
        personalize_knn_search_result(
            &storage,
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
            let history = trim_history(
                history,
                AsRef::<PersonalizationConfig>::as_ref(config).max_stateless_history_for_cois,
            );
            let history = load_history(storage, history).await?;
            derive_interests_and_tag_weights(coi_system, &history)
        }
    };

    if interests.len() >= AsRef::<CoiConfig>::as_ref(config).min_cois() {
        rerank_by_scores(
            coi_system,
            documents,
            &interests,
            &tag_weights,
            AsRef::<SemanticSearchConfig>::as_ref(config).score_weights,
            Utc::now(),
        );
    }

    Ok(())
}
