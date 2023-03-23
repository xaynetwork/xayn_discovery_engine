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
use itertools::Itertools;
use serde::{de, Deserialize, Deserializer, Serialize};
use tokio::time::Instant;
use tracing::{error, info, instrument};

use super::AppState;
#[cfg(feature = "ET-4089")]
use crate::{error::common::FailedToSetSomeDocumentCandidates, models::DocumentId};
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{
            BadRequest,
            DocumentNotFound,
            DocumentPropertyNotFound,
            FailedToDeleteSomeDocuments,
            IngestingDocumentsFailed,
        },
    },
    models::{self, DocumentProperties, DocumentProperty},
    storage,
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    config.service(
        web::resource("/documents")
            .route(web::post().to(new_documents.error_with_request_id()))
            .route(web::delete().to(delete_documents.error_with_request_id())),
    );
    #[cfg(feature = "ET-4089")]
    config.service(
        web::resource("/documents/candidates")
            .route(web::get().to(get_document_candidates.error_with_request_id()))
            .route(web::put().to(set_document_candidates.error_with_request_id())),
    );
    config
        .service(
            web::resource("/documents/{document_id}")
                .route(web::delete().to(delete_document.error_with_request_id())),
        )
        .service(
            web::resource("/documents/{document_id}/properties")
                .route(web::get().to(get_document_properties.error_with_request_id()))
                .route(web::put().to(put_document_properties.error_with_request_id()))
                .route(web::delete().to(delete_document_properties.error_with_request_id())),
        )
        .service(
            web::resource("/documents/{document_id}/properties/{property_id}")
                .route(web::get().to(get_document_property.error_with_request_id()))
                .route(web::put().to(put_document_property.error_with_request_id()))
                .route(web::delete().to(delete_document_property.error_with_request_id())),
        );
}

fn deserialize_string_not_empty_or_zero_bytes<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Err(de::Error::custom("string is empty"))
    } else if s.contains('\u{0000}') {
        Err(de::Error::custom("string contains zero bytes"))
    } else {
        Ok(s)
    }
}

#[cfg(feature = "ET-4089")]
const fn default_is_candidate() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct IngestedDocument {
    id: String,
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_bytes")]
    snippet: String,
    #[serde(default)]
    properties: HashMap<String, DocumentProperty>,
    #[serde(default)]
    tags: Vec<String>,
    #[cfg(feature = "ET-4089")]
    #[serde(default = "default_is_candidate")]
    is_candidate: bool,
}

/// Represents body of a POST documents request.
#[derive(Debug, Deserialize)]
struct IngestionRequestBody {
    documents: Vec<IngestedDocument>,
}

#[instrument(skip_all)]
async fn new_documents(
    state: Data<AppState>,
    Json(body): Json<IngestionRequestBody>,
) -> Result<impl Responder, Error> {
    if body.documents.is_empty() {
        return Ok(HttpResponse::NoContent());
    }

    if body.documents.len() > state.config.ingestion.max_document_batch_size {
        error!("{} documents exceeds maximum number", body.documents.len());
        return Err(BadRequest::from(format!(
            "Document batch size exceeded maximum of {}.",
            state.config.ingestion.max_document_batch_size
        ))
        .into());
    }

    let start = Instant::now();

    let (documents, mut failed_documents) = body
        .documents
        .into_iter()
        .map(|document| {
            let map_document = || -> anyhow::Result<_> {
                let id = document.id.as_str().try_into()?;
                let properties = document
                    .properties
                    .into_iter()
                    .map(|(id, property)| id.try_into().map(|id| (id, property)))
                    .try_collect()?;
                let tags = document
                    .tags
                    .into_iter()
                    .map(TryInto::try_into)
                    .try_collect()?;
                let embedding = state.embedder.run(&document.snippet)?;

                Ok(models::IngestedDocument {
                    id,
                    snippet: document.snippet,
                    properties,
                    tags,
                    embedding,
                    #[cfg(feature = "ET-4089")]
                    is_candidate: document.is_candidate,
                })
            };

            map_document().map_err(|error| {
                error!(
                    "Document with id '{}' caused a PipelineError: {:#?}",
                    document.id, error,
                );
                document.id.into()
            })
        })
        .partition_result::<Vec<_>, Vec<_>, _, _>();

    info!(
        "{} embeddings calculated in {} sec",
        documents.len(),
        start.elapsed().as_secs(),
    );

    failed_documents.extend(
        storage::Document::insert(&state.storage, documents)
            .await?
            .into_iter()
            .map(Into::into),
    );

    if failed_documents.is_empty() {
        Ok(HttpResponse::Created())
    } else {
        Err(IngestingDocumentsFailed {
            documents: failed_documents,
        }
        .into())
    }
}

async fn delete_document(state: Data<AppState>, id: Path<String>) -> Result<impl Responder, Error> {
    delete_documents(
        state,
        Json(BatchDeleteRequest {
            documents: vec![id.into_inner()],
        }),
    )
    .await?;

    Ok(HttpResponse::NoContent())
}

async fn delete_documents(
    state: Data<AppState>,
    Json(documents): Json<BatchDeleteRequest>,
) -> Result<impl Responder, Error> {
    let documents = documents
        .documents
        .into_iter()
        .map(TryInto::try_into)
        .try_collect::<_, Vec<_>, _>()?;
    let failed_documents = storage::Document::delete(&state.storage, &documents).await?;

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

#[cfg(feature = "ET-4089")]
#[derive(Debug, Serialize)]
struct DocumentCandidatesResponse {
    documents: Vec<DocumentId>,
}

#[cfg(feature = "ET-4089")]
async fn get_document_candidates(state: Data<AppState>) -> Result<impl Responder, Error> {
    let documents = storage::DocumentCandidate::get(&state.storage).await?;

    Ok(Json(DocumentCandidatesResponse { documents }))
}

#[cfg(feature = "ET-4089")]
#[derive(Debug, Deserialize)]
struct DocumentCandidate {
    id: String,
}

#[cfg(feature = "ET-4089")]
#[derive(Debug, Deserialize)]
struct DocumentCandidatesRequest {
    documents: Vec<DocumentCandidate>,
}

#[cfg(feature = "ET-4089")]
async fn set_document_candidates(
    state: Data<AppState>,
    Json(body): Json<DocumentCandidatesRequest>,
) -> Result<impl Responder, Error> {
    let documents = body
        .documents
        .into_iter()
        .map(|document| document.id.try_into())
        .try_collect::<_, Vec<_>, _>()?;
    let failed_documents = storage::DocumentCandidate::set(&state.storage, &documents).await?;

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

#[instrument(skip(state))]
pub(crate) async fn get_document_properties(
    state: Data<AppState>,
    document_id: Path<String>,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let properties = storage::DocumentProperties::get(&state.storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(Json(DocumentPropertiesResponse { properties }))
}

#[derive(Debug, Deserialize)]
struct DocumentPropertiesRequest {
    properties: HashMap<String, DocumentProperty>,
}

#[instrument(skip(state, properties))]
async fn put_document_properties(
    state: Data<AppState>,
    document_id: Path<String>,
    Json(properties): Json<DocumentPropertiesRequest>,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    let properties = properties
        .properties
        .into_iter()
        .map(|(id, property)| id.try_into().map(|id| (id, property)))
        .try_collect()?;
    storage::DocumentProperties::put(&state.storage, &document_id, &properties)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(state))]
async fn delete_document_properties(
    state: Data<AppState>,
    document_id: Path<String>,
) -> Result<impl Responder, Error> {
    let document_id = document_id.into_inner().try_into()?;
    storage::DocumentProperties::delete(&state.storage, &document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[derive(Debug, Serialize)]
struct DocumentPropertyResponse {
    property: DocumentProperty,
}

#[instrument(skip(state))]
async fn get_document_property(
    state: Data<AppState>,
    ids: Path<(String, String)>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = property_id.try_into()?;
    let property = storage::DocumentProperty::get(&state.storage, &document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?
        .ok_or(DocumentPropertyNotFound)?;

    Ok(Json(DocumentPropertyResponse { property }))
}

#[derive(Debug, Deserialize)]
struct DocumentPropertyRequest {
    property: DocumentProperty,
}

#[instrument(skip(state))]
async fn put_document_property(
    state: Data<AppState>,
    ids: Path<(String, String)>,
    Json(body): Json<DocumentPropertyRequest>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = property_id.try_into()?;
    storage::DocumentProperty::put(&state.storage, &document_id, &property_id, &body.property)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(state))]
async fn delete_document_property(
    state: Data<AppState>,
    ids: Path<(String, String)>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let document_id = document_id.try_into()?;
    let property_id = property_id.try_into()?;
    storage::DocumentProperty::delete(&state.storage, &document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?
        .ok_or(DocumentPropertyNotFound)?;

    Ok(HttpResponse::NoContent())
}
