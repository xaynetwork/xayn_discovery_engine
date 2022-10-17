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

use std::{convert::Infallible, sync::Arc};
use warp::{self, Filter, Rejection, Reply};

use crate::{
    handlers,
    models::{
        DocumentId,
        DocumentPropertyId,
        Error,
        PersonalizedDocumentsQuery,
        UserId,
        COUNT_PARAM_RANGE,
    },
    state::AppState,
};

pub fn api_routes(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_personalized_documents(state.clone())
        .or(patch_user_interactions(state.clone()))
        .or(get_document_properties(state.clone()))
        .or(put_document_properties(state.clone()))
        .or(delete_document_properties(state.clone()))
        .or(get_document_property(state))
}

// GET /users/:user_id/personalized_documents
fn get_personalized_documents(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(user_path())
        .and(warp::path("personalized_documents"))
        .and(with_count_param())
        .and(with_state(state))
        .and_then(handlers::handle_personalized_documents)
}

// PATCH /users/:user_id/interactions
fn patch_user_interactions(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::patch()
        .and(user_path())
        .and(warp::path("interactions"))
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(handlers::handle_user_interactions)
}

// PATH /users/:user_id
fn user_path() -> impl Filter<Extract = (UserId,), Error = Rejection> + Clone {
    let user_id_param = warp::path::param().and_then(|user_id: String| async move {
        urlencoding::decode(&user_id)
            .map_err(Error::UserIdUtf8Conversion)
            .and_then(UserId::new)
            .map_err(warp::reject::custom)
    });

    warp::path("users").and(user_id_param)
}

// GET /documents/:document_id/properties
fn get_document_properties(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(document_properties_path())
        .and(with_state(state))
        .and_then(handlers::handle_get_document_properties)
}

// PUT /documents/:document_id/properties
fn put_document_properties(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::put()
        .and(document_properties_path())
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(handlers::handle_put_document_properties)
}

// DELETE /documents/:document_id/properties
fn delete_document_properties(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(document_properties_path())
        .and(with_state(state))
        .and_then(handlers::handle_delete_document_properties)
}

// GET /documents/:document_id/properties/:property_id
fn get_document_property(
    state: Arc<AppState>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(document_property_path())
        .and(with_state(state))
        .and_then(handlers::handle_get_document_property)
}

// PATH /documents/:document_id/properties
fn document_properties_path() -> impl Filter<Extract = (DocumentId,), Error = Rejection> + Clone {
    let document_id_param = warp::path::param().and_then(|document_id: String| async move {
        urlencoding::decode(&document_id)
            .map_err(Error::DocumentIdUtf8Conversion)
            .and_then(DocumentId::new)
            .map_err(warp::reject::custom)
    });

    warp::path("documents")
        .and(document_id_param)
        .and(warp::path("properties"))
}

// PATH /documents/:document_id/properties/:property_id
fn document_property_path(
) -> impl Filter<Extract = (DocumentId, DocumentPropertyId), Error = Rejection> + Clone {
    let property_id_param = warp::path::param().and_then(|property_id: String| async move {
        urlencoding::decode(&property_id)
            .map_err(Error::DocumentPropertyIdUtf8Conversion)
            .and_then(DocumentPropertyId::new)
            .map_err(warp::reject::custom)
    });

    document_properties_path().and(property_id_param)
}

/// Extract a "count" from query params and check if within bounds, or reject with InvalidCountParam error.
fn with_count_param(
) -> impl Filter<Extract = (PersonalizedDocumentsQuery,), Error = Rejection> + Copy {
    warp::query().and_then(|query: PersonalizedDocumentsQuery| async {
        match query.count {
            Some(count) if COUNT_PARAM_RANGE.contains(&count) => Ok(query),
            Some(count) => Err(warp::reject::custom(Error::InvalidCountParam(count))),
            None => Ok(query),
        }
    })
}

fn with_state(
    state: Arc<AppState>,
) -> impl Filter<Extract = (Arc<AppState>,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}
