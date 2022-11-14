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
use serde::{de, Deserialize, Deserializer, Serialize};
use tokio::time::Instant;
use tracing::{error, info, instrument};

use crate::{
    elastic::{BulkInsertionError, ElasticDocument},
    error::{
        application::WithRequestIdExt,
        common::{
            BadRequest,
            DocumentNotFound,
            DocumentPropertyNotFound,
            IngestingDocumentsFailed,
        },
    },
    models::{DocumentId, DocumentProperties, DocumentProperty, DocumentPropertyId},
    Error,
};

use super::AppState;

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
struct IngestionRequestBody {
    documents: Vec<IngestedDocument>,
}

/// Represents a document sent for ingestion.
#[derive(Debug, Clone, Deserialize)]
struct IngestedDocument {
    /// Unique identifier of the document.
    id: DocumentId,

    /// Snippet used to calculate embeddings for a document.
    #[serde(deserialize_with = "deserialize_string_not_empty_or_zero_byte")]
    snippet: String,

    /// Contents of the document properties.
    properties: DocumentProperties,

    /// The high-level category the document belongs to.
    #[serde(default, deserialize_with = "deserialize_empty_option_string_as_none")]
    category: Option<String>,
}

fn deserialize_string_not_empty_or_zero_byte<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.is_empty() {
        Err(de::Error::custom("field can't be an empty string"))
    } else if s.contains('\u{0000}') {
        Err(de::Error::custom("field can't contain zero bytes"))
    } else {
        Ok(s)
    }
}

fn deserialize_empty_option_string_as_none<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|s| s.filter(|s| !s.is_empty()))
}

#[instrument(skip(state))]
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
            Ok(embedding) => Ok((
                document.id,
                ElasticDocument {
                    snippet: document.snippet,
                    properties: document.properties,
                    embedding,
                    category: document.category,
                },
            )),
            Err(err) => {
                error!(
                    "Document with id '{}' caused a PipelineError: {:#?}",
                    document.id, err
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
        .elastic
        .bulk_insert_documents(&documents)
        .await
        .map_err(|err| match err {
            BulkInsertionError::General(err) => err,
            BulkInsertionError::PartialFailure {
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
    do_delete_documents(&state, vec![id.into_inner()]).await?;
    Ok(HttpResponse::NoContent())
}

async fn delete_documents(
    state: Data<AppState>,
    documents: Json<BatchDeleteRequest>,
) -> Result<impl Responder, Error> {
    do_delete_documents(&state, documents.into_inner().documents).await?;
    Ok(HttpResponse::NoContent())
}

#[derive(Deserialize)]
struct BatchDeleteRequest {
    documents: Vec<DocumentId>,
}

async fn do_delete_documents(state: &AppState, documents: Vec<DocumentId>) -> Result<(), Error> {
    state.db.delete_documents(&documents).await?;
    state.elastic.delete_documents(&documents).await?;
    Ok(())
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
        .elastic
        .get_document_properties(&document_id)
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
        .elastic
        .put_document_properties(&document_id, &properties.properties)
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
        .elastic
        .delete_document_properties(&document_id)
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
        .elastic
        .get_document_property(&document_id, &property_id)
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
        .elastic
        .put_document_property(&document_id, &property_id, &body.property)
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
        .elastic
        .delete_document_property(&document_id, &property_id)
        .await?
        .ok_or(DocumentNotFound)?;

    Ok(HttpResponse::NoContent())
}
