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
    storage::{self, DeletionError, InsertionError},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    config
        .service(
            web::resource("/documents")
                .route(web::post().to(new_documents.error_with_request_id()))
                .route(web::delete().to(delete_documents.error_with_request_id())),
        )
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

#[derive(Debug, Deserialize)]
struct IngestedDocument {
    id: String,
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_bytes")]
    snippet: String,
    #[serde(default)]
    properties: HashMap<String, DocumentProperty>,
    #[serde(default)]
    tags: Vec<String>,
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
                let document = models::IngestedDocument {
                    id: document.id.as_str().try_into()?,
                    snippet: document.snippet,
                    properties: document
                        .properties
                        .into_iter()
                        .map(|(id, property)| id.try_into().map(|id| (id, property)))
                        .try_collect()?,
                    tags: document
                        .tags
                        .into_iter()
                        .map(TryInto::try_into)
                        .try_collect()?,
                };
                let embedding = state.embedder.run(&document.snippet)?;

                Ok((document, embedding))
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

    storage::Document::insert(&state.storage, documents)
        .await
        .map_err(|err| match err {
            InsertionError::General(err) => err,
            InsertionError::PartialFailure {
                failed_documents: fd,
            } => {
                failed_documents.extend(fd);
                IngestingDocumentsFailed {
                    documents: failed_documents,
                }
                .into()
            }
        })?;

    Ok(HttpResponse::Created())
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
    storage::Document::delete(&state.storage, &documents)
        .await
        .map_err(|error| match error {
            DeletionError::General(error) => error,
            DeletionError::PartialFailure { errors } => {
                FailedToDeleteSomeDocuments { errors }.into()
            }
        })?;

    Ok(HttpResponse::NoContent())
}

#[derive(Debug, Deserialize)]
struct BatchDeleteRequest {
    documents: Vec<String>,
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
