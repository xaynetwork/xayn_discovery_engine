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

use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};

use futures::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use tracing::{debug, error, instrument};
use warp::{http::StatusCode, Reply};

use xayn_discovery_engine_ai::{
    compute_coi_relevances,
    nan_safe_f32_cmp,
    system_time_now,
    utils::rank,
    PositiveCoi,
};

use crate::{
    elastic::KnnSearchParams,
    models::{
        DocumentId,
        DocumentPropertiesRequestBody,
        DocumentPropertiesResponse,
        Error,
        PersonalizedDocumentsError,
        PersonalizedDocumentsErrorKind,
        PersonalizedDocumentsQuery,
        PersonalizedDocumentsResponse,
        UserId,
        UserInteractionError,
        UserInteractionErrorKind,
        UserInteractionRequestBody,
        UserInteractionType,
    },
    state::AppState,
};

#[instrument(skip(state))]
pub(crate) async fn handle_personalized_documents(
    user_id: UserId,
    query: PersonalizedDocumentsQuery,
    state: Arc<AppState>,
) -> Result<Box<dyn Reply>, Infallible> {
    if let Err(err) = state.user.user_seen(&user_id).await {
        error!("Error updating user seen: {err}");
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR) as Box<dyn Reply>);
    }

    let user_interests = match state.user.fetch_interests(&user_id).await {
        Ok(user_interests) => user_interests,
        Err(error) => {
            error!("Error fetching interests: {error}");
            return Ok(Box::new(StatusCode::BAD_REQUEST) as Box<dyn Reply>);
        }
    };

    if user_interests.is_empty() {
        error!("No user interests");
        return Ok(Box::new(StatusCode::NOT_FOUND));
    }

    let cois = &user_interests.positive;
    let horizon = state.coi.config().horizon();
    let coi_weights = compute_coi_weights(cois, horizon);
    let cois = cois
        .iter()
        .zip(coi_weights)
        .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
        .collect_vec();

    let max_cois = state.max_cois_for_knn.min(user_interests.positive.len());
    let cois = &cois[0..max_cois];
    let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

    let excluded = match state.user.fetch_interacted_document_ids(&user_id).await {
        Ok(excluded) => excluded,
        Err(error) => {
            error!("Error fetching interacted document ids: {error}");
            return Ok(Box::new(StatusCode::BAD_REQUEST));
        }
    };
    let documents_count = query.count.unwrap_or(state.default_documents_count);
    let mut document_futures = cois
        .iter()
        .map(|(coi, weight)| async {
            // weights_sum can't be zero, because coi weights will always return some weights that are > 0
            let weight = *weight / weights_sum;
            #[allow(
                // fine as max documents count is small enough
                clippy::cast_precision_loss,
                // fine as weight should be between 0 and 1
                clippy::cast_sign_loss,
                // fine as number of neighbors is small enough
                clippy::cast_possible_truncation
            )]
            let k_neighbors = (weight * documents_count as f32).ceil() as usize;

            state
                .elastic
                .get_documents_by_embedding(KnnSearchParams {
                    excluded: excluded.clone(),
                    embedding: coi.point.to_vec(),
                    size: k_neighbors,
                    k_neighbors,
                    num_candidates: documents_count,
                })
                .await
        })
        .collect::<FuturesUnordered<_>>();

    let mut all_documents = Vec::new();
    let mut errors = Vec::new();

    while let Some(result) = document_futures.next().await {
        match result {
            Ok(documents) => all_documents.extend(documents),
            Err(err) => {
                error!("Error fetching document: {err}");
                errors.push(err);
            }
        };
    }

    if all_documents.is_empty() && !errors.is_empty() {
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR));
    }

    match state.coi.score(&all_documents, &user_interests) {
        Ok(scores) => rank(&mut all_documents, &scores),
        Err(error) => {
            error!("Error scoring documents: {error}");
            return Ok(Box::new(
                PersonalizedDocumentsError::new(
                    PersonalizedDocumentsErrorKind::NotEnoughInteractions,
                )
                .to_reply(StatusCode::UNPROCESSABLE_ENTITY),
            ));
        }
    }

    let max_docs = documents_count.min(all_documents.len());
    let documents = &all_documents[0..max_docs];

    Ok(Box::new(
        PersonalizedDocumentsResponse::new(documents).to_reply(),
    ))
}

#[instrument(skip(state))]
pub(crate) async fn handle_user_interactions(
    user_id: UserId,
    body: UserInteractionRequestBody,
    state: Arc<AppState>,
) -> Result<Box<dyn Reply>, Infallible> {
    if let Err(err) = state.user.user_seen(&user_id).await {
        error!("Error updating user seen: {err}");
        return Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR) as Box<dyn Reply>);
    }

    let ids = body
        .documents
        .iter()
        .map(|document| &document.document_id)
        .collect_vec();
    let embeddings = match state.elastic.get_documents_by_ids(&ids).await {
        Ok(documents) => documents
            .into_iter()
            .map(|document| (document.id, document.embedding))
            .collect::<HashMap<_, _>>(),
        Err(error) => {
            error!("Error fetching documents: {error}");
            return Ok(Box::new(
                UserInteractionError::new(UserInteractionErrorKind::InvalidDocumentId)
                    .to_reply(StatusCode::BAD_REQUEST),
            ) as Box<dyn Reply>);
        }
    };

    if embeddings.len() < ids.iter().unique().count() {
        debug!("Document not found");
        return Ok(Box::new(
            UserInteractionError::new(UserInteractionErrorKind::InvalidDocumentId)
                .to_reply(StatusCode::BAD_REQUEST),
        ));
    }

    for document in body.documents {
        match document.interaction_type {
            UserInteractionType::Positive => {
                if let Err(error) = state
                    .user
                    .update_positive_cois(&document.document_id, &user_id, |positive_cois| {
                        state.coi.log_positive_user_reaction(
                            positive_cois,
                            &embeddings[&document.document_id],
                        )
                    })
                    .await
                {
                    error!("Error updating positive user interests: {error}");
                    return Ok(Box::new(
                        UserInteractionError::new(UserInteractionErrorKind::InvalidUserId)
                            .to_reply(StatusCode::BAD_REQUEST),
                    ));
                }
            }
        }
    }

    Ok(Box::new(StatusCode::NO_CONTENT))
}

/// Computes [`PositiveCoi`]s weights used to determine how many documents to fetch using each centers' embedding.
fn compute_coi_weights(cois: &[PositiveCoi], horizon: Duration) -> Vec<f32> {
    let relevances = compute_coi_relevances(cois, horizon, system_time_now())
        .into_iter()
        .map(|rel| 1.0 - (-3.0 * rel).exp())
        .collect_vec();

    let rel_sum: f32 = relevances.iter().sum();
    relevances
        .iter()
        .map(|rel| {
            let res = rel / rel_sum;
            if res.is_nan() {
                // should be ok for our use-case
                #[allow(clippy::cast_precision_loss)]
                let len = cois.len() as f32;
                // len can't be zero, because we return early if we have no positive CoIs and never compute weights
                1.0f32 / len
            } else {
                res
            }
        })
        .collect()
}

#[instrument(skip(state))]
pub(crate) async fn handle_get_document_properties(
    doc_id: DocumentId,
    state: Arc<AppState>,
) -> Result<Box<dyn Reply>, Infallible> {
    match state.elastic.get_document_properties(&doc_id).await {
        Ok(properties) => Ok(Box::new(DocumentPropertiesResponse::new(properties).to_reply()) as _),
        Err(Error::Elastic(error)) if matches!(error.status(), Some(StatusCode::NOT_FOUND)) => {
            Ok(Box::new(StatusCode::NOT_FOUND) as _)
        }
        Err(error) => {
            error!("Error fetching document properties: {error}");
            Ok(Box::new(StatusCode::BAD_REQUEST) as _)
        }
    }
}

#[instrument(skip(state))]
pub(crate) async fn handle_put_document_properties(
    doc_id: DocumentId,
    body: DocumentPropertiesRequestBody,
    state: Arc<AppState>,
) -> Result<StatusCode, Infallible> {
    match state
        .elastic
        .put_document_properties(&doc_id, &body.properties)
        .await
    {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(Error::Elastic(error)) if matches!(error.status(), Some(StatusCode::NOT_FOUND)) => {
            Ok(StatusCode::NOT_FOUND)
        }
        Err(error) => {
            error!("Error fetching document properties: {error}");
            Ok(StatusCode::BAD_REQUEST)
        }
    }
}
