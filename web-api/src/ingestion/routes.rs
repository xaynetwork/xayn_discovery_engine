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
    web::{self, Data, Json, Path, ServiceConfig},
    HttpResponse,
    Responder,
};
use anyhow::anyhow;
use chrono::DateTime;
use itertools::{Either, Itertools};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::Instant;
use tracing::{debug, error, info, instrument};
use xayn_summarizer::{summarize, Config, Source, Summarizer};
use xayn_web_api_db_ctrl::{Operation, Silo};

use super::AppState;
use crate::{
    app::TenantState,
    error::common::{
        BadRequest,
        DocumentNotFound,
        DocumentPropertyNotFound,
        FailedToDeleteSomeDocuments,
        FailedToIngestDocuments,
        FailedToSetSomeDocumentCandidates,
        FailedToValidateDocuments,
        InvalidDocumentProperty,
    },
    models::{
        self,
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        DocumentSnippet,
        DocumentTags,
    },
    storage::{self, property_filter::IndexedPropertiesSchemaUpdate, IndexedProperties},
    Error,
};

// https://datatracker.ietf.org/doc/html/draft-dalal-deprecation-header-00
macro_rules! deprecate {
    ($fn:ident($($args:tt)*)) => {
        |$($args)*| async {
            $fn($($args)*).await.customize().append_header((
                ::actix_web::http::header::HeaderName::from_static("deprecation"),
                ::actix_web::http::header::HeaderValue::from_static("version=\"current\""),
            ))
        }
    };
}

pub(super) fn configure_service(config: &mut ServiceConfig) {
    config
        .service(
            web::resource("/documents")
                .route(web::post().to(upsert_documents))
                .route(web::delete().to(delete_documents)),
        )
        .service(
            web::resource("/documents/_candidates")
                .route(web::get().to(get_document_candidates))
                .route(web::put().to(set_document_candidates)),
        )
        .service(
            web::resource("/documents/_indexed_properties")
                .route(web::post().to(create_indexed_properties)),
        )
        .service(web::resource("/documents/{document_id}").route(web::delete().to(delete_document)))
        .service(
            web::resource("/documents/{document_id}/properties")
                .route(web::get().to(get_document_properties))
                .route(web::put().to(put_document_properties))
                .route(web::delete().to(delete_document_properties)),
        )
        .service(
            web::resource("/documents/{document_id}/properties/{property_id}")
                .route(web::get().to(get_document_property))
                .route(web::put().to(put_document_property))
                .route(web::delete().to(delete_document_property)),
        )
        // all routes below are deprecated and undocumented and will be removed in the future
        .service(
            web::resource("/candidates")
                .route(web::get().to(deprecate!(get_document_candidates(state))))
                .route(web::put().to(deprecate!(set_document_candidates(request, state)))),
        )
        .service(
            web::resource("/documents/candidates")
                .route(web::get().to(deprecate!(get_document_candidates(state))))
                .route(web::put().to(deprecate!(set_document_candidates(request, state)))),
        );
}

pub(super) fn configure_ops_service(config: &mut ServiceConfig) {
    config.service(web::resource("/silo_management").route(web::post().to(silo_management)));
}

#[derive(Debug, Deserialize)]
struct UnvalidatedIngestedDocument {
    id: String,
    snippet: String,
    #[serde(default)]
    properties: HashMap<String, Value>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    is_candidate: Option<bool>,
    #[serde(default)]
    default_is_candidate: Option<bool>,
    #[serde(default)]
    summarize: bool,
}

#[derive(Debug)]
struct IngestedDocument {
    id: DocumentId,
    snippet: DocumentSnippet,
    properties: DocumentProperties,
    tags: DocumentTags,
    is_candidate_op: IsCandidateOp,
}

#[derive(Clone, Debug, Copy)]
enum IsCandidateOp {
    SetTo(bool),
    DefaultTo(bool),
}

impl IsCandidateOp {
    /// Returns a `NewIsCandidate` instance.
    fn resolve(self, existing: Option<bool>) -> NewIsCandidate {
        match self {
            IsCandidateOp::SetTo(new) => NewIsCandidate {
                value: new,
                existing_and_has_changed: existing
                    .map(|previous| previous != new)
                    .unwrap_or_default(),
            },
            IsCandidateOp::DefaultTo(default) => NewIsCandidate {
                value: existing.unwrap_or(default),
                existing_and_has_changed: false,
            },
        }
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(PartialEq, Debug))]
struct NewIsCandidate {
    /// The new value of `is_candidate`.
    value: bool,
    /// `true` if there had been an existing value for `is_candidate` and it differs from the new value
    existing_and_has_changed: bool,
}

impl NewIsCandidate {
    fn has_changed_to_false(self) -> bool {
        self.existing_and_has_changed && !self.value
    }

    fn has_changed_to_true(self) -> bool {
        self.existing_and_has_changed && self.value
    }
}

async fn validate_document_properties(
    properties: impl IntoIterator<Item = (String, Value)>,
    storage: &impl storage::Size,
) -> Result<DocumentProperties, Error> {
    let properties = properties
        .into_iter()
        .map(|(id, property)| Ok::<_, Error>((id.try_into()?, property.try_into()?)))
        .try_collect::<_, HashMap<_, DocumentProperty>, _>()?;
    if let Some(publication_date) = properties.get("publication_date") {
        let Some(publication_date) = publication_date.as_str() else {
            return Err(InvalidDocumentProperty { value: publication_date.clone().into() }.into());
        };
        DateTime::parse_from_rfc3339(publication_date)?;
    }
    let size = storage::Size::json(storage, &serde_json::to_value(&properties)?).await?;

    DocumentProperties::new(properties, size).map_err(Into::into)
}

impl UnvalidatedIngestedDocument {
    async fn validate(self, storage: &impl storage::Size) -> Result<IngestedDocument, Error> {
        let id = self.id.as_str().try_into()?;
        let snippet = if self.summarize {
            summarize(
                &Summarizer::Naive,
                &Source::PlainText { text: self.snippet },
                &Config::default(),
            )
        } else {
            self.snippet
        }
        .try_into()?;
        let properties = validate_document_properties(self.properties, storage).await?;
        let tags = self
            .tags
            .into_iter()
            .map(TryInto::try_into)
            .try_collect::<_, Vec<_>, _>()?
            .try_into()?;

        let is_candidate_op = match (self.is_candidate, self.default_is_candidate) {
            (Some(value), None) => IsCandidateOp::SetTo(value),
            (None, Some(value)) => IsCandidateOp::DefaultTo(value),
            (None, None) => IsCandidateOp::SetTo(true),
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "You can only use either of is_candidate or default_is_candidate"
                )
                .into());
            }
        };

        Ok(IngestedDocument {
            id,
            snippet,
            properties,
            tags,
            is_candidate_op,
        })
    }
}

/// Represents body of a POST documents request.
#[derive(Debug, Deserialize)]
struct IngestionRequestBody {
    documents: Vec<UnvalidatedIngestedDocument>,
}

#[instrument(skip_all)]
async fn upsert_documents(
    state: Data<AppState>,
    Json(body): Json<IngestionRequestBody>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    if body.documents.is_empty() {
        return Ok(HttpResponse::NoContent());
    }

    if body.documents.len() > state.config.ingestion.max_document_batch_size {
        info!("{} documents exceeds maximum number", body.documents.len());
        return Err(BadRequest::from(format!(
            "Document batch size exceeded maximum of {}.",
            state.config.ingestion.max_document_batch_size
        ))
        .into());
    }

    let mut documents = Vec::with_capacity(body.documents.len());
    let mut invalid_documents = Vec::new();
    for document in body.documents {
        let id = document.id.clone();
        match UnvalidatedIngestedDocument::validate(document, &storage).await {
            Ok(document) => documents.push(document),
            Err(error) => {
                info!("Invalid document '{id}': {error:#?}");
                invalid_documents.push(id.into());
            }
        }
    }

    let ids = documents.iter().enumerate().fold(
        HashMap::with_capacity(documents.len()),
        |mut ids, (index, document)| {
            ids.insert(document.id.clone(), index);
            ids
        },
    );
    if ids.len() != documents.len() {
        documents = documents
            .into_iter()
            .enumerate()
            .filter_map(|(index, document)| (ids[&document.id] == index).then_some(document))
            .collect();
    };

    let existing_documents =
        storage::Document::get_excerpted(&storage, documents.iter().map(|document| &document.id))
            .await?
            .into_iter()
            .map(|document| {
                (
                    document.id,
                    (
                        document.snippet,
                        document.properties,
                        document.tags,
                        document.is_candidate,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

    // Hint: Documents which have a changed snippet are also in `new_documents`.
    let (new_documents, changed_documents) = documents
        .into_iter()
        .partition_map::<Vec<_>, Vec<_>, _, _, _>(|document| {
            let (data, is_candidate) = existing_documents
                .get(&document.id)
                .map(|(snippet, properties, tags, is_candidate)| {
                    ((snippet, properties, tags), *is_candidate)
                })
                .unzip();

            let new_snippet = data.map_or(true, |(snippet, _, _)| snippet != &document.snippet);
            let new_is_candidate = document.is_candidate_op.resolve(is_candidate);

            if new_snippet {
                Either::Left((document, new_is_candidate))
            } else {
                let new_properties = data.map_or(true, |(_, properties, _)| {
                    properties != &document.properties
                });
                let new_tags = data.map_or(true, |(_, _, tags)| tags != &document.tags);
                Either::Right((document, new_properties, new_tags, new_is_candidate))
            }
        });

    storage::DocumentCandidate::remove(
        &storage,
        changed_documents
            .iter()
            .filter_map(|(document, _, _, new_is_candidate)| {
                new_is_candidate
                    .has_changed_to_false()
                    .then_some(&document.id)
            }),
    )
    .await?;

    for (document, new_properties, new_tags, _) in &changed_documents {
        if *new_properties {
            storage::DocumentProperties::put(&storage, &document.id, &document.properties).await?;
        }
        if *new_tags {
            storage::Tag::put(&storage, &document.id, &document.tags).await?;
        }
    }

    storage::DocumentCandidate::add(
        &storage,
        changed_documents
            .iter()
            .filter_map(|(document, _, _, new_is_candidate)| {
                new_is_candidate
                    .has_changed_to_true()
                    .then_some(&document.id)
            }),
    )
    .await?;

    let start = Instant::now();
    let (new_documents, mut failed_documents) = new_documents
        .into_iter()
        .partition_map::<Vec<_>, Vec<_>, _, _, _>(|(document, new_is_candidate)| {
            match state.embedder.run(&document.snippet) {
                Ok(embedding) => Either::Left(models::IngestedDocument {
                    id: document.id,
                    snippet: document.snippet,
                    properties: document.properties,
                    tags: document.tags,
                    embedding,
                    is_candidate: new_is_candidate.value,
                }),
                Err(error) => {
                    error!("Failed to embed document '{}': {:#?}", document.id, error);
                    Either::Right(document.id.into())
                }
            }
        });

    debug!(
        "{} new embeddings calculated in {} seconds and {} unchanged embeddings skipped",
        new_documents.len(),
        start.elapsed().as_secs(),
        changed_documents.len(),
    );
    failed_documents.extend(
        storage::Document::insert(&storage, new_documents)
            .await?
            .into_iter()
            .map(Into::into),
    );

    if !failed_documents.is_empty() {
        failed_documents.extend(invalid_documents);
        Err(FailedToIngestDocuments {
            documents: failed_documents,
        }
        .into())
    } else if !invalid_documents.is_empty() {
        Err(FailedToValidateDocuments {
            documents: invalid_documents,
        }
        .into())
    } else {
        Ok(HttpResponse::Created())
    }
}

async fn delete_document(id: Path<String>, state: TenantState) -> Result<impl Responder, Error> {
    delete_documents(
        Json(BatchDeleteRequest {
            documents: vec![id.into_inner()],
        }),
        state,
    )
    .await?;

    Ok(HttpResponse::NoContent())
}

async fn delete_documents(
    Json(documents): Json<BatchDeleteRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let documents = documents
        .documents
        .into_iter()
        .map(TryInto::try_into)
        .try_collect::<_, Vec<_>, _>()?;
    let failed_documents = storage::Document::delete(&storage, &documents).await?;

    if failed_documents.is_empty() {
        Ok(HttpResponse::NoContent())
    } else {
        Err(FailedToDeleteSomeDocuments {
            errors: failed_documents.into_iter().map(Into::into).collect(),
        }
        .into())
    }
}

#[derive(Debug, Deserialize)]
struct BatchDeleteRequest {
    documents: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DocumentCandidatesResponse {
    documents: Vec<DocumentId>,
}

async fn get_document_candidates(
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let documents = storage::DocumentCandidate::get(&storage).await?;

    Ok(Json(DocumentCandidatesResponse { documents }))
}

#[derive(Debug, Deserialize)]
struct DocumentCandidate {
    id: String,
}

#[derive(Debug, Deserialize)]
struct DocumentCandidatesRequest {
    documents: Vec<DocumentCandidate>,
}

async fn set_document_candidates(
    Json(body): Json<DocumentCandidatesRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let documents = body
        .documents
        .into_iter()
        .map(|document| document.id.try_into())
        .try_collect::<_, Vec<_>, _>()?;
    let failed_documents = storage::DocumentCandidate::set(&storage, &documents).await?;

    if failed_documents.is_empty() {
        Ok(HttpResponse::NoContent())
    } else {
        Err(FailedToSetSomeDocumentCandidates {
            documents: failed_documents.into_iter().map(Into::into).collect(),
        }
        .into())
    }
}

#[derive(Debug, Serialize)]
struct DocumentPropertiesResponse {
    properties: DocumentProperties,
}

#[instrument(skip(storage))]
pub(crate) async fn get_document_properties(
    document_id: Path<String>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let properties = storage::DocumentProperties::get(&storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(Json(DocumentPropertiesResponse { properties }))
}

#[derive(Debug, Deserialize)]
struct DocumentPropertiesRequest {
    properties: HashMap<String, Value>,
}

#[instrument(skip(properties, storage))]
async fn put_document_properties(
    document_id: Path<String>,
    Json(properties): Json<DocumentPropertiesRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let properties = validate_document_properties(properties.properties, &storage).await?;
    storage::DocumentProperties::put(&storage, &document_id, &properties)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(storage))]
async fn delete_document_properties(
    document_id: Path<String>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    storage::DocumentProperties::delete(&storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[derive(Debug, Serialize)]
struct DocumentPropertyResponse {
    property: DocumentProperty,
}

#[instrument(skip(storage))]
async fn get_document_property(
    ids: Path<(String, String)>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = property_id.try_into()?;
    let property = storage::DocumentProperty::get(&storage, &document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?
        .ok_or(DocumentPropertyNotFound)?;

    Ok(Json(DocumentPropertyResponse { property }))
}

#[derive(Debug, Deserialize)]
struct DocumentPropertyRequest {
    property: Value,
}

#[instrument(skip(storage))]
async fn put_document_property(
    ids: Path<(String, String)>,
    Json(body): Json<DocumentPropertyRequest>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = DocumentPropertyId::try_from(property_id)?;
    let property = DocumentProperty::try_from(body.property)?;

    let properties = storage::DocumentProperties::get(&storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?
        .into_iter()
        .chain([(property_id.clone(), property.clone())])
        .map(|(property_id, property)| (property_id.into(), property.into()));
    validate_document_properties(properties, &storage).await?;

    storage::DocumentProperty::put(&storage, &document_id, &property_id, &property)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(storage))]
async fn delete_document_property(
    ids: Path<(String, String)>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = property_id.try_into()?;
    storage::DocumentProperty::delete(&storage, &document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?
        .ok_or(DocumentPropertyNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(storage))]
async fn create_indexed_properties(
    Json(update): Json<IndexedPropertiesSchemaUpdate>,
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    // TODO[pmk/now] max property count from congfig
    Ok(Json(
        IndexedProperties::extend_schema(&storage, update, 11).await?,
    ))
}

#[instrument(skip(storage))]
async fn get_indexed_properties_schema(
    TenantState(storage): TenantState,
) -> Result<impl Responder, Error> {
    Ok(Json(IndexedProperties::load_schema(&storage).await?))
}

#[derive(Deserialize, Debug)]
struct ManagementRequest {
    operations: Vec<Operation>,
}

#[instrument(skip(silo))]
async fn silo_management(
    Json(request): Json<ManagementRequest>,
    silo: Data<Silo>,
) -> Result<impl Responder, Error> {
    let results = silo.run_operations(false, request.operations).await?;
    let results = results
        .iter()
        .map(serde_json::to_value)
        .try_collect::<_, Vec<_>, _>()?;

    Ok(Json(json!({ "results": results })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_is_candidate_op() {
        for (op, existing, (value, existing_and_has_changed)) in [
            (IsCandidateOp::SetTo(true), None, (true, false)),
            (IsCandidateOp::SetTo(true), Some(false), (true, true)),
            (IsCandidateOp::SetTo(true), Some(true), (true, false)),
            (IsCandidateOp::SetTo(false), None, (false, false)),
            (IsCandidateOp::SetTo(false), Some(false), (false, false)),
            (IsCandidateOp::SetTo(false), Some(true), (false, true)),
            (IsCandidateOp::DefaultTo(true), None, (true, false)),
            (IsCandidateOp::DefaultTo(true), Some(false), (false, false)),
            (IsCandidateOp::DefaultTo(true), Some(true), (true, false)),
            (IsCandidateOp::DefaultTo(false), None, (false, false)),
            (IsCandidateOp::DefaultTo(false), Some(false), (false, false)),
            (IsCandidateOp::DefaultTo(false), Some(true), (true, false)),
        ] {
            assert_eq!(
                op.resolve(existing),
                NewIsCandidate {
                    value,
                    existing_and_has_changed
                }
            );
        }
    }
}
