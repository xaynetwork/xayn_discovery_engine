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
use serde::Deserialize;

use crate::{
    error::application::{Unimplemented, WithRequestIdExt},
    models::DocumentId,
    Error,
};

use super::AppState;

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let documents = web::scope("/documents")
        .service(
            web::resource("")
                .route(web::post().to(new_documents.error_with_request_id()))
                .route(web::delete().to(delete_documents.error_with_request_id())),
        )
        .service(
            web::resource("/{document_id}")
                .route(web::delete().to(delete_document.error_with_request_id())),
        );
    config.service(documents);
}

//FIXME use actual body
#[derive(Deserialize)]
struct NewDocuments {}

async fn new_documents(
    _state: Data<AppState>,
    _new_documents: Json<NewDocuments>,
) -> Result<impl Responder, Error> {
    if true {
        Err(Unimplemented {
            functionality: "endpoint /documents",
        })?;
    }
    Ok("text body response")
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
    state.elastic.delete_documents(documents).await?;
    Ok(())
    // //TODO: We currently don't have postgres access in the ingestion endpoint so we can't delete
    // //       from postgres.
    // let url = format!(
    //     "{}/{}/_doc/{}",
    //     config.elastic_url,
    //     config.elastic_index_name,
    //     urlencoding::encode(id)
    // );

    // let response = client
    //     .delete(url)
    //     .basic_auth(&config.elastic_user, Some(&config.elastic_password))
    //     .send()
    //     .await
    //     .map_err(|err| {
    //         error!("Connecting to elastic failed: {}", err);
    //         warp::reject::custom(ElasticSearchFailed)
    //     })?;

    // if response.status() == StatusCode::NOT_FOUND {
    //     return Ok(());
    // }

    // response.error_for_status().map_err(|error| {
    //     error!("Elastic returned unexpected status code: {}", error);
    //     warp::reject::custom(ElasticSearchFailed)
    // })?;
}
