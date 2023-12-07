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
    web::{Data, Json},
    Responder,
};
use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use xayn_ai_coi::{CoiConfig, CoiSystem};

use super::super::{
    filter::Filter,
    rerank::rerank,
    stateless::{derive_interests_and_tag_weights, load_history, trim_history},
    PersonalizationConfig,
    SemanticSearchConfig,
};
use crate::{
    app::{AppState, TenantState},
    embedding::EmbeddingKind,
    error::{
        common::{BadRequest, DocumentNotFound, ForbiddenDevOption},
        warning::Warning,
    },
    frontoffice::shared::{
        default_include_properties,
        personalized_exclusions,
        validate_count,
        InputUser,
        Personalize,
        UnvalidatedPersonalize,
        UnvalidatedSnippetOrDocumentId,
    },
    models::{
        DocumentDevData,
        DocumentId,
        DocumentProperties,
        DocumentQuery,
        DocumentSnippet,
        PersonalizedDocument,
        SnippetId,
        SnippetOrDocumentId,
    },
    storage::{self, Exclusions, KnnSearchParams, MergeFn, NormalizationFn, SearchStrategy},
    tenants,
    utils::deprecate,
    Error,
};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum DevHybrid {
    Customize {
        normalize_knn: NormalizationFn,
        normalize_bm25: NormalizationFn,
        merge_fn: MergeFn,
    },
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct DevOption {
    hybrid: Option<DevHybrid>,
    max_number_candidates: Option<usize>,
    show_raw_scores: Option<bool>,
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

#[derive(Debug, Serialize)]
pub(super) struct PersonalizedDocumentData {
    id: DocumentId,
    snippet_id: SnippetId,
    score: f32,
    #[serde(skip_serializing_if = "no_properties")]
    properties: Option<DocumentProperties>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snippet: Option<DocumentSnippet>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dev: Option<DocumentDevData>,
}

impl From<PersonalizedDocument> for PersonalizedDocumentData {
    fn from(document: PersonalizedDocument) -> Self {
        Self {
            id: document.id.document_id().clone(),
            snippet_id: document.id,
            score: document.score,
            properties: document.properties,
            snippet: document.snippet,
            dev: document.dev,
        }
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
    dev_show_raw_scores: Option<bool>,
    include_properties: bool,
    include_snippet: bool,
    filter: Option<Filter>,
    is_deprecated: bool,
}

#[derive(Serialize)]
pub(super) struct SemanticSearchResponse {
    pub(crate) documents: Vec<PersonalizedDocumentData>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UnvalidatedSemanticSearchRequest {
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

        let document = document.validate(semantic_search_config)?;
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
        let dev_show_raw_scores = dev.show_raw_scores;
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
            dev_show_raw_scores,
            include_properties,
            include_snippet,
            filter,
            is_deprecated,
        })
    }
}

enum InputDocument {
    DocumentId(DocumentId),
    SnippetId(SnippetId),
    Query(DocumentQuery),
}

impl From<SnippetOrDocumentId> for InputDocument {
    fn from(value: SnippetOrDocumentId) -> Self {
        match value {
            SnippetOrDocumentId::SnippetId(id) => InputDocument::SnippetId(id),
            SnippetOrDocumentId::DocumentId(id) => InputDocument::DocumentId(id),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UnvalidatedInputDocument {
    id: Option<UnvalidatedSnippetOrDocumentId>,
    query: Option<String>,
}

impl UnvalidatedInputDocument {
    fn validate(self, config: &SemanticSearchConfig) -> Result<InputDocument, Error> {
        let id = self
            .id
            .map(|id| id.validate().map(InputDocument::from))
            .transpose()?;
        match (id, self.query) {
            (Some(_), Some(_)) => Err(BadRequest::from(
                "either id or query must be present in the request, but both were found",
            )
            .into()),
            (None, Some(query)) => Ok(InputDocument::Query(
                DocumentQuery::new_with_length_constraint(query, config.query_size_bounds())?,
            )),
            (Some(id), None) => Ok(id),
            (None, None) => {
                Err(BadRequest::from("either id or query must be present in the request").into())
            }
        }
    }
}

fn no_properties(properties: &Option<DocumentProperties>) -> bool {
    properties
        .as_ref()
        .map_or(true, |properties| properties.is_empty())
}

#[instrument(skip(state, storage, embedder))]
pub(super) async fn semantic_search(
    state: Data<AppState>,
    Json(body): Json<UnvalidatedSemanticSearchRequest>,
    TenantState(storage, embedder): TenantState,
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
        dev_show_raw_scores,
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
            let embedding = embedder.run(EmbeddingKind::Query, query).await?;
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
            with_raw_scores: dev_show_raw_scores.unwrap_or(false),
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

    Ok(deprecate!(if is_deprecated {
        Json(SemanticSearchResponse {
            documents: documents.into_iter().map_into().collect(),
        })
    }))
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
