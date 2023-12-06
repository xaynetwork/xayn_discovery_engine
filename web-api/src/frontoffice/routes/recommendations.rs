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

use actix_web::{
    http::StatusCode,
    web::{Data, Json, Path, Query},
    Either,
    Responder,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::Deserialize;
use tracing::instrument;

use super::{PersonalizationConfig, SemanticSearchConfig};
use crate::{
    app::{AppState, TenantState},
    error::warning::Warning,
    frontoffice::{
        filter::Filter,
        knn,
        rerank::rerank,
        routes::semantic_search::SemanticSearchResponse,
        shared::{
            default_include_properties,
            personalized_exclusions,
            validate_count,
            InputUser,
            Personalize,
            PersonalizedDocumentsError,
            UnvalidatedPersonalize,
        },
        stateless::{derive_interests_and_tag_weights, load_history, trim_history},
    },
    models::UserId,
    storage::{self, Storage},
    tenants,
    utils::deprecate,
    Error,
};

struct RecommendationRequest {
    count: usize,
    personalize: Personalize,
    include_properties: bool,
    include_snippet: bool,
    filter: Option<Filter>,
    is_deprecated: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedRecommendationRequest {
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
    personalize: UnvalidatedPersonalize,
    #[serde(default = "default_include_properties")]
    include_properties: bool,
    #[serde(default)]
    include_snippet: bool,
    filter: Option<Filter>,
}

impl UnvalidatedRecommendationRequest {
    async fn validate_and_resolve_defaults(
        self,
        config: &(impl AsRef<SemanticSearchConfig>
              + AsRef<PersonalizationConfig>
              + AsRef<tenants::Config>),
        storage: &impl storage::IndexedProperties,
        warnings: &mut Vec<Warning>,
    ) -> Result<RecommendationRequest, Error> {
        let Self {
            count,
            published_after,
            personalize,
            include_properties,
            include_snippet,
            filter,
        } = self;

        let semantic_search_config: &SemanticSearchConfig = config.as_ref();

        let count = count.unwrap_or(semantic_search_config.default_number_documents);
        validate_count(
            count,
            semantic_search_config.max_number_documents,
            semantic_search_config.max_number_candidates,
        )?;

        let personalize = personalize.validate(config.as_ref(), warnings)?;
        // let history = validate_history(history, personalize_config, warnings, Utc::now(), false)?;
        let filter = Filter::insert_published_after(filter, published_after);
        if let Some(filter) = &filter {
            filter.validate(&storage.load_schema().await?)?;
        }
        let is_deprecated = published_after.is_some();

        Ok(RecommendationRequest {
            count,
            personalize,
            include_properties,
            include_snippet,
            filter,
            is_deprecated,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedPersonalizedDocumentsRequest {
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
    filter: Option<Filter>,
    #[serde(default = "default_include_properties")]
    include_properties: bool,
    #[serde(default)]
    include_snippet: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedPersonalizedDocumentsQuery {
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
    filter: Option<String>,
    #[serde(default = "default_include_properties")]
    include_properties: bool,
    #[serde(default)]
    include_snippet: bool,
}

impl UnvalidatedPersonalizedDocumentsRequest {
    async fn validate_and_resolve_defaults(
        self,
        config: &impl AsRef<PersonalizationConfig>,
        storage: &impl storage::IndexedProperties,
        user_id: UserId,
    ) -> Result<RecommendationRequest, Error> {
        let Self {
            count,
            published_after,
            filter,
            include_properties,
            include_snippet,
        } = self;
        let config = config.as_ref();

        let count = count.unwrap_or(config.default_number_documents);
        validate_count(
            count,
            config.max_number_documents,
            config.max_number_candidates,
        )?;
        let filter = Filter::insert_published_after(filter, published_after);
        if let Some(filter) = &filter {
            filter.validate(&storage.load_schema().await?)?;
        }
        let is_deprecated = published_after.is_some();

        let personalize = Personalize {
            exclude_seen: true,
            user: InputUser::Ref { id: user_id },
        };

        Ok(RecommendationRequest {
            count,
            personalize,
            include_properties,
            include_snippet,
            filter,
            is_deprecated,
        })
    }
}

#[instrument(skip(state, storage))]
pub(super) async fn recommendations(
    state: Data<AppState>,
    Json(body): Json<UnvalidatedRecommendationRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    // TODO: actually return non-empty warnings in the response
    let mut warnings = Vec::new();
    let request = body
        .validate_and_resolve_defaults(&state.config, &storage, &mut warnings)
        .await?;

    recommendations_inner(state, request, storage).await
}

async fn recommendations_inner(
    state: Data<AppState>,
    request: RecommendationRequest,
    storage: Storage,
) -> Result<impl Responder, Error> {
    let RecommendationRequest {
        count,
        personalize,
        include_properties,
        include_snippet,
        filter,
        is_deprecated,
    } = request;

    let time = Utc::now();
    let exclusions = personalized_exclusions(&storage, state.config.as_ref(), &personalize).await?;

    let (interests, tag_weights) = match personalize.user {
        InputUser::Ref { id } => {
            storage::Interaction::user_seen(&storage, &id, time).await?;
            (
                storage::Interest::get(&storage, &id).await?,
                storage::Tag::get(&storage, &id).await?,
            )
        }
        InputUser::Inline { history } => {
            let history = trim_history(
                history,
                state.config.personalization.max_stateless_history_for_cois,
            );
            let history = load_history(&storage, history).await?;
            derive_interests_and_tag_weights(&state.coi, &history)
        }
    };

    if interests.len() < state.coi.config().min_cois() {
        return Ok(Either::Left((
            deprecate!(if is_deprecated {
                Json(PersonalizedDocumentsError::NotEnoughInteractions)
            }),
            StatusCode::CONFLICT,
        )));
    }

    let mut documents = knn::CoiSearch {
        interests: &interests,
        excluded: &exclusions,
        horizon: state.coi.config().horizon(),
        max_cois: state.config.personalization.max_cois_for_knn,
        count,
        num_candidates: state.config.personalization.max_number_candidates,
        time,
        include_properties,
        include_snippet,
        filter: filter.as_ref(),
    }
    .run_on(&storage)
    .await?;

    rerank(
        &state.coi,
        &mut documents,
        &interests,
        &tag_weights,
        state.config.personalization.score_weights,
        time,
    );

    if documents.len() > count {
        // due to ceiling the number of documents we fetch per COI
        // we might end up with more documents than we want
        documents.truncate(count);
    }

    Ok(Either::Right(deprecate!(if is_deprecated {
        Json(SemanticSearchResponse {
            documents: documents.into_iter().map_into().collect(),
        })
    })))
}

pub(super) async fn user_recommendations(
    state: Data<AppState>,
    user_id: Path<String>,
    body: Option<Json<UnvalidatedPersonalizedDocumentsRequest>>,
    Query(params): Query<UnvalidatedPersonalizedDocumentsQuery>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let request: RecommendationRequest = if let Some(Json(body)) = body {
        body.validate_and_resolve_defaults(&state.config, &storage, user_id)
            .await?
    } else {
        UnvalidatedPersonalizedDocumentsRequest {
            count: params.count,
            published_after: params.published_after,
            filter: params
                .filter
                .map(|filter| serde_json::from_str(&filter))
                .transpose()?,
            include_properties: params.include_properties,
            include_snippet: params.include_snippet,
        }
        .validate_and_resolve_defaults(&state.config, &storage, user_id)
        .await?
        // TODO: once the deprecated params are removed use this instead in case of no request body
        // PersonalizedDocumentsRequest {
        //     count: state.config.personalization.default_number_documents,
        //     filter: None,
        //     include_properties: default_include_properties(),
        //     is_deprecated: false,
        // }
    };
    recommendations_inner(state, request, storage).await
}
