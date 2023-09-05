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
    filter::Filter,
    knn,
    rerank::rerank,
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
        common::{BadRequest, DocumentNotFound, ForbiddenDevOption, InvalidDocumentCount},
        warning::Warning,
    },
    models::{
        DocumentId,
        DocumentProperties,
        DocumentQuery,
        DocumentSnippet,
        PersonalizedDocument,
        SnippetId,
        SnippetOrDocumentId,
        UserId,
    },
    storage::{self, Exclusions, KnnSearchParams, MergeFn, NormalizationFn, SearchStrategy},
    tenants,
    utils::deprecate,
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let users = web::scope("/users/{user_id}")
        .service(web::resource("interactions").route(web::patch().to(interactions)))
        .service(
            web::resource("personalized_documents")
                .route(web::post().to(personalized_documents))
                // this route is deprecated and will be removed in the future
                .route(web::get().to(deprecate!(personalized_documents(
                    state, user_id, body, params, storage,
                )))),
        );
    let semantic_search = web::resource("/semantic_search").route(web::post().to(semantic_search));

    config.service(users).service(semantic_search);
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedUserInteractionRequest {
    documents: Vec<UnvalidatedUserInteraction>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedUserInteraction {
    id: UnvalidatedSnippetOrDocumentId,
}

impl UnvalidatedUserInteractionRequest {
    fn validate(self) -> Result<Vec<SnippetOrDocumentId>, Error> {
        self.documents
            .into_iter()
            .map(|document| document.id.validate())
            .try_collect()
    }
}

async fn interactions(
    state: Data<AppState>,
    user_id: Path<String>,
    Json(body): Json<UnvalidatedUserInteractionRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let interactions = body.validate()?;
    update_interactions(
        &storage,
        &state.coi,
        &user_id,
        interactions,
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

const fn default_include_properties() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedPersonalizedDocumentsRequest {
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
struct UnvalidatedPersonalizedDocumentsQuery {
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
    filter: Option<String>,
    #[serde(default = "default_include_properties")]
    include_properties: bool,
    #[serde(default)]
    include_snippet: bool,
}

#[derive(Debug)]
struct PersonalizedDocumentsRequest {
    count: usize,
    filter: Option<Filter>,
    include_properties: bool,
    include_snippet: bool,
    is_deprecated: bool,
}

impl UnvalidatedPersonalizedDocumentsRequest {
    async fn validate_and_resolve_defaults(
        self,
        config: &impl AsRef<PersonalizationConfig>,
        storage: &impl storage::IndexedProperties,
    ) -> Result<PersonalizedDocumentsRequest, Error> {
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

        Ok(PersonalizedDocumentsRequest {
            count,
            filter,
            include_properties,
            include_snippet,
            is_deprecated,
        })
    }
}

async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<String>,
    body: Option<Json<UnvalidatedPersonalizedDocumentsRequest>>,
    Query(params): Query<UnvalidatedPersonalizedDocumentsQuery>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let user_id = user_id.into_inner().try_into()?;
    let PersonalizedDocumentsRequest {
        count,
        filter,
        include_properties,
        include_snippet,
        is_deprecated,
    } = if let Some(Json(body)) = body {
        body.validate_and_resolve_defaults(&state.config, &storage)
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
        .validate_and_resolve_defaults(&state.config, &storage)
        .await?
        // TODO: once the deprecated params are removed use this instead in case of no request body
        // PersonalizedDocumentsRequest {
        //     count: state.config.personalization.default_number_documents,
        //     filter: None,
        //     include_properties: default_include_properties(),
        //     is_deprecated: false,
        // }
    };

    let documents = if let Some(documents) = personalize_documents_by(
        &storage,
        &state.coi,
        &user_id,
        &state.config.personalization,
        PersonalizeBy::KnnSearch {
            count,
            filter: filter.as_ref(),
        },
        Utc::now(),
        include_properties,
        include_snippet,
    )
    .await?
    {
        Either::Left(Json(PersonalizedDocumentsResponse {
            documents: documents.into_iter().map_into().collect(),
        }))
    } else {
        Either::Right((
            Json(PersonalizedDocumentsError::NotEnoughInteractions),
            StatusCode::CONFLICT,
        ))
    };

    Ok(deprecate!(if is_deprecated {
        documents
    }))
}

#[derive(Debug, Serialize)]
struct PersonalizedDocumentData {
    id: DocumentId,
    snippet_id: SnippetId,
    score: f32,
    #[serde(skip_serializing_if = "no_properties")]
    properties: Option<DocumentProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<DocumentSnippet>,
}

fn no_properties(properties: &Option<DocumentProperties>) -> bool {
    properties
        .as_ref()
        .map_or(true, |properties| properties.is_empty())
}

impl From<PersonalizedDocument> for PersonalizedDocumentData {
    fn from(document: PersonalizedDocument) -> Self {
        Self {
            id: document.id.document_id().clone(),
            snippet_id: document.id,
            score: document.score,
            properties: document.properties,
            snippet: document.snippet,
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
        filter: Option<&'a Filter>,
    },
    #[cfg(test)]
    Documents(&'a [&'a DocumentId]),
}

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
        #[cfg(test)]
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

    normalize_knn_scores(&mut documents);
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
struct UnvalidatedSemanticSearchRequest {
    document: UnvalidatedInputDocument,
    count: Option<usize>,
    published_after: Option<DateTime<Utc>>,
    personalize: Option<UnvalidatedPersonalize>,
    #[serde(default)]
    enable_hybrid_search: bool,
    #[serde(default, rename = "_dev")]
    dev: DevOption,
    #[serde(default = "default_include_properties")]
    include_properties: bool,
    #[serde(default)]
    include_snippet: bool,
    filter: Option<Filter>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct DevOption {
    hybrid: Option<DevHybrid>,
    max_number_candidates: Option<usize>,
}

impl DevOption {
    fn validate(&self, enable_dev: bool) -> Result<(), Error> {
        if !enable_dev && (self.hybrid.is_some() || self.max_number_candidates.is_some()) {
            // notify the caller instead of silently discarding the dev option
            return Err(ForbiddenDevOption::DevDisabled.into());
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum DevHybrid {
    Customize {
        normalize_knn: NormalizationFn,
        normalize_bm25: NormalizationFn,
        merge_fn: MergeFn,
    },
}

impl<'a> SearchStrategy<'a> {
    fn new(
        enable_hybrid_search: bool,
        dev_hybrid_search: Option<DevHybrid>,
        query: Option<&'a DocumentQuery>,
    ) -> Self {
        if !enable_hybrid_search {
            return Self::Knn;
        }
        let Some(query) = query else {
            return Self::Knn;
        };
        let Some(dev_hybrid_search) = dev_hybrid_search else {
            return Self::Hybrid { query };
        };

        match dev_hybrid_search {
            DevHybrid::Customize {
                normalize_knn,
                normalize_bm25,
                merge_fn,
            } => Self::HybridDev {
                query,
                normalize_knn,
                normalize_bm25,
                merge_fn,
            },
        }
    }
}

impl UnvalidatedSemanticSearchRequest {
    async fn validate_and_resolve_defaults(
        self,
        config: &(impl AsRef<SemanticSearchConfig>
              + AsRef<PersonalizationConfig>
              + AsRef<tenants::Config>),
        storage: &impl storage::IndexedProperties,
        warnings: &mut Vec<Warning>,
    ) -> Result<SemanticSearchRequest, Error> {
        let Self {
            document,
            count,
            published_after,
            personalize,
            enable_hybrid_search,
            dev,
            include_properties,
            include_snippet,
            filter,
        } = self;
        let semantic_search_config: &SemanticSearchConfig = config.as_ref();
        let tenants_config: &tenants::Config = config.as_ref();

        let document = document.validate()?;
        let count = count.unwrap_or(semantic_search_config.default_number_documents);
        dev.validate(tenants_config.enable_dev)?;
        let num_candidates = dev
            .max_number_candidates
            .unwrap_or(semantic_search_config.max_number_candidates);
        validate_count(
            count,
            semantic_search_config.max_number_documents,
            num_candidates,
        )?;
        let personalize = personalize
            .map(|personalize| personalize.validate(config.as_ref(), warnings))
            .transpose()?;
        let dev_hybrid_search = dev.hybrid;
        let filter = Filter::insert_published_after(filter, published_after);
        if let Some(filter) = &filter {
            filter.validate(&storage.load_schema().await?)?;
        }
        let is_deprecated = published_after.is_some();

        Ok(SemanticSearchRequest {
            document,
            count,
            num_candidates,
            personalize,
            enable_hybrid_search,
            dev_hybrid_search,
            include_properties,
            include_snippet,
            filter,
            is_deprecated,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub(super) enum UnvalidatedSnippetOrDocumentId {
    DocumentId(String),
    SnippetId { document_id: String, sub_id: u32 },
}

impl From<SnippetOrDocumentId> for InputDocument {
    fn from(value: SnippetOrDocumentId) -> Self {
        match value {
            SnippetOrDocumentId::SnippetId(id) => InputDocument::SnippetId(id),
            SnippetOrDocumentId::DocumentId(id) => InputDocument::DocumentId(id),
        }
    }
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
struct UnvalidatedInputDocument {
    id: Option<UnvalidatedSnippetOrDocumentId>,
    query: Option<String>,
}

impl UnvalidatedInputDocument {
    fn validate(self) -> Result<InputDocument, Error> {
        let id = self
            .id
            .map(|id| id.validate().map(InputDocument::from))
            .transpose()?;
        match (id, self.query) {
            (Some(_), Some(_)) => Err(BadRequest::from(
                "either id or query must be present in the request, but both were found",
            )
            .into()),
            (None, Some(query)) => Ok(InputDocument::Query(query.try_into()?)),
            (Some(id), None) => Ok(id),
            (None, None) => {
                Err(BadRequest::from("either id or query must be present in the request").into())
            }
        }
    }
}

const fn default_exclude_seen() -> bool {
    true
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnvalidatedPersonalize {
    #[serde(default = "default_exclude_seen")]
    exclude_seen: bool,
    user: UnvalidatedInputUser,
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

#[allow(clippy::too_many_arguments)]
#[allow(clippy::struct_excessive_bools)]
struct SemanticSearchRequest {
    document: InputDocument,
    count: usize,
    num_candidates: usize,
    personalize: Option<Personalize>,
    enable_hybrid_search: bool,
    dev_hybrid_search: Option<DevHybrid>,
    include_properties: bool,
    include_snippet: bool,
    filter: Option<Filter>,
    is_deprecated: bool,
}

enum InputDocument {
    DocumentId(DocumentId),
    SnippetId(SnippetId),
    Query(DocumentQuery),
}

struct Personalize {
    exclude_seen: bool,
    user: InputUser,
}

fn validate_count(count: usize, max: usize, candidates: usize) -> Result<(), InvalidDocumentCount> {
    let min = 1;
    let max = max.min(candidates);
    if !(min..=max).contains(&count) {
        return Err(InvalidDocumentCount { count, min, max });
    }

    Ok(())
}

#[derive(Serialize)]
struct SemanticSearchResponse {
    documents: Vec<PersonalizedDocumentData>,
}

#[instrument(skip(state, storage))]
async fn semantic_search(
    state: Data<AppState>,
    Json(body): Json<UnvalidatedSemanticSearchRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    // TODO: actually return non-empty warnings in the response
    let mut warnings = Vec::new();
    let SemanticSearchRequest {
        document,
        count,
        num_candidates,
        personalize,
        enable_hybrid_search,
        dev_hybrid_search,
        include_properties,
        include_snippet,
        filter,
        is_deprecated,
    } = body
        .validate_and_resolve_defaults(&state.config, &storage, &mut warnings)
        .await?;

    let mut exclusions = if let Some(personalize) = &personalize {
        personalized_exclusions(&storage, state.config.as_ref(), personalize).await?
    } else {
        Exclusions::default()
    };
    let (embedding, query) = match document {
        InputDocument::DocumentId(id) => {
            // TODO[pmk/ET-4933] how to handle by document search with multi-snippet documents
            let id = SnippetId::new(id, 0);
            let embedding = storage::Document::get_embedding(&storage, &id)
                .await?
                .ok_or(DocumentNotFound)?;
            exclusions.documents.push(id.into_document_id());
            (embedding, None)
        }
        InputDocument::SnippetId(id) => {
            let embedding = storage::Document::get_embedding(&storage, &id)
                .await?
                .ok_or(DocumentNotFound)?;
            exclusions.snippets.push(id);
            (embedding, None)
        }
        InputDocument::Query(ref query) => {
            let embedding = state.embedder.run(query).await?;
            (embedding, Some(query))
        }
    };
    let strategy = SearchStrategy::new(enable_hybrid_search, dev_hybrid_search, query);

    let mut documents = storage::Document::get_by_embedding(
        &storage,
        KnnSearchParams {
            excluded: &exclusions,
            embedding: &embedding,
            count,
            num_candidates,
            strategy,
            include_properties,
            include_snippet,
            filter: filter.as_ref(),
        },
    )
    .await?;

    if let Some(personalize) = personalize {
        if matches!(strategy, SearchStrategy::Knn) {
            normalize_knn_scores(&mut documents);
        }
        personalize_knn_search_result(
            &storage,
            &state.config,
            &state.coi,
            personalize,
            &mut documents,
        )
        .await?;
    }

    Ok(deprecate!(if is_deprecated {
        Json(SemanticSearchResponse {
            documents: documents.into_iter().map_into().collect(),
        })
    }))
}

async fn personalized_exclusions(
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
        rerank(
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

/// Normalize knn similarity scores for `rerank_by_scores()`.
fn normalize_knn_scores(documents: &mut [PersonalizedDocument]) {
    for document in documents {
        document.score = (document.score + 1.) / 2.;
    }
}
