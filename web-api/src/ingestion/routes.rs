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
    web::{self, Data, Json, Path, ServiceConfig},
    HttpResponse,
    Responder,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
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
    models::{
        DocumentId,
        DocumentProperties,
        DocumentProperty,
        DocumentPropertyId,
        IngestedDocument,
    },
    storage::{
        DeletionError,
        Document as _,
        DocumentProperties as _,
        DocumentProperty as _,
        InsertionError,
    },
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

/// Represents body of a POST documents request.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct IngestionRequestBody {
    pub(crate) documents: Vec<IngestedDocument>,
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
        .map(|document| match state.embedder.run(&document.snippet) {
            Ok(embedding) => Ok((document, embedding)),
            Err(err) => {
                error!(
                    "Document with id '{}' caused a PipelineError: {:#?}",
                    document.id, err,
                );
                Err(document.id.into())
            }
        })
        .partition_result::<Vec<_>, Vec<_>, _, _>();

    info!(
        "{} embeddings calculated in {} sec",
        documents.len(),
        start.elapsed().as_secs(),
    );

    state
        .storage
        .document()
        .insert(documents)
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

    Ok(HttpResponse::NoContent())
}

async fn delete_document(
    state: Data<AppState>,
    id: Path<DocumentId>,
) -> Result<impl Responder, Error> {
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
    state
        .storage
        .document()
        .delete(&documents.documents)
        .await
        .map_err(|error| match error {
            DeletionError::General(error) => error,
            DeletionError::PartialFailure { errors } => {
                FailedToDeleteSomeDocuments { errors }.into()
            }
        })?;

    Ok(HttpResponse::NoContent())
}

#[derive(Deserialize)]
struct BatchDeleteRequest {
    documents: Vec<DocumentId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct DocumentPropertiesAsObject {
    properties: DocumentProperties,
}

#[instrument(skip(state))]
pub(crate) async fn get_document_properties(
    state: Data<AppState>,
    document_id: Path<DocumentId>,
) -> Result<impl Responder, Error> {
    let properties = state
        .storage
        .document_properties()
        .get(&document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(Json(DocumentPropertiesAsObject { properties }))
}

#[instrument(skip(state, properties))]
async fn put_document_properties(
    state: Data<AppState>,
    document_id: Path<DocumentId>,
    Json(properties): Json<DocumentPropertiesAsObject>,
) -> Result<impl Responder, Error> {
    state
        .storage
        .document_properties()
        .put(&document_id, &properties.properties)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(state))]
async fn delete_document_properties(
    state: Data<AppState>,
    document_id: Path<DocumentId>,
) -> Result<impl Responder, Error> {
    state
        .storage
        .document_properties()
        .delete(&document_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DocumentPropertyAsObject {
    property: DocumentProperty,
}

#[instrument(skip(state))]
async fn get_document_property(
    state: Data<AppState>,
    ids: Path<(DocumentId, DocumentPropertyId)>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    let property = state
        .storage
        .document_property()
        .get(&document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?
        .ok_or(DocumentPropertyNotFound)?;

    Ok(Json(DocumentPropertyAsObject { property }))
}

#[instrument(skip(state))]
async fn put_document_property(
    state: Data<AppState>,
    ids: Path<(DocumentId, DocumentPropertyId)>,
    Json(body): Json<DocumentPropertyAsObject>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    state
        .storage
        .document_property()
        .put(&document_id, &property_id, &body.property)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}

#[instrument(skip(state))]
async fn delete_document_property(
    state: Data<AppState>,
    ids: Path<(DocumentId, DocumentPropertyId)>,
) -> Result<impl Responder, Error> {
    let (document_id, property_id) = ids.into_inner();
    state
        .storage
        .document_property()
        .delete(&document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}
