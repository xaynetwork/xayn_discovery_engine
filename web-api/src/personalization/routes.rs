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

use std::{cmp::Ordering, collections::HashMap, time::Duration};

use actix_web::{
    body::EitherBody,
    web::{self, Data, Json, Path, Query, ServiceConfig},
    HttpResponse,
    Responder,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use xayn_ai_coi::{
    compute_coi_relevances,
    nan_safe_f32_cmp,
    system_time_now,
    utils::rank,
    PositiveCoi,
};

use super::PersonalizationConfig;
#[cfg(feature = "mind")]
use crate::mind::AppState;
#[cfg(not(feature = "mind"))]
use crate::personalization::AppState;
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{BadRequest, InternalError, NotEnoughInteractions},
    },
    models::{DocumentId, PersonalizedDocument, UserId, UserInteractionType},
    storage::{Category as _, Document as _, Interaction as _, Interest as _, KnnSearchParams},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let scope = web::scope("/users/{user_id}")
        .service(
            web::resource("interactions")
                .route(web::patch().to(update_interactions.error_with_request_id())),
        )
        .service(
            web::resource("personalized_documents")
                .route(web::get().to(personalized_documents.error_with_request_id())),
        );

    config.service(scope);
}

/// Represents user interaction request body.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UpdateInteractions {
    pub(crate) documents: Vec<UserInteractionData>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct UserInteractionData {
    #[serde(rename = "id")]
    pub(crate) document_id: DocumentId,
    #[serde(rename = "type")]
    pub(crate) interaction_type: UserInteractionType,
}

pub(crate) async fn update_interactions(
    state: Data<AppState>,
    user_id: Path<UserId>,
    Json(interactions): Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    state.storage.interaction().user_seen(&user_id).await?;

    let ids = interactions
        .documents
        .iter()
        .map(|document| &document.document_id)
        .collect_vec();
    let documents = state.storage.document().get_by_ids(&ids).await?;
    let documents = documents
        .iter()
        .map(|document| (&document.id, document))
        .collect::<HashMap<_, _>>();

    for document in interactions.documents {
        match document.interaction_type {
            UserInteractionType::Positive => {
                if let Some(document) = documents.get(&document.document_id) {
                    state
                        .storage
                        .interest()
                        .update_positive(&document.id, &user_id, |positive_cois| {
                            state
                                .coi
                                .log_positive_user_reaction(positive_cois, &document.embedding)
                        })
                        .await?;
                    if let Some(category) = &document.category {
                        state.storage.category().update(&user_id, category).await?;
                    }
                } else {
                    warn!(%document.document_id, "interacted document doesn't exist anymore");
                }
            }
        }
    }

    Ok(HttpResponse::NoContent())
}

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    pub(crate) count: Option<usize>,
    /// Personalize only the specified documents, otherwise fetch documents to be personalized.
    // Note: documents can't be provided via the public server api, since Query<Self> always fails
    // to deserialize if documents is present in a query string (because it's a sequence) and the
    // route returns a bad request
    pub(crate) documents: Option<Vec<DocumentId>>,
}

impl PersonalizedDocumentsQuery {
    fn document_count(&self, config: &PersonalizationConfig) -> Result<usize, Error> {
        let count = self.count.map_or(config.default_number_documents, |count| {
            count.min(config.max_number_documents)
        });

        if count > 0 {
            Ok(count)
        } else {
            Err(BadRequest::from("count has to be at least 1").into())
        }
    }
}

pub(crate) async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<UserId>,
    Query(options): Query<PersonalizedDocumentsQuery>,
) -> Result<impl Responder<Body = EitherBody<String>>, Error> {
    let document_count = options.document_count(&state.config.personalization)?;

    state.storage.interaction().user_seen(&user_id).await?;

    let user_interests = state.storage.interest().get(&user_id).await?;

    if user_interests.is_empty() {
        return Err(NotEnoughInteractions.into());
    }

    let mut all_documents = if let Some(documents) = options.documents {
        state
            .storage
            .document()
            .get_by_ids(&documents.iter().collect_vec())
            .await?
    } else {
        search_knn_documents(&state, &user_id, &user_interests.positive, document_count).await?
    };

    match state.coi.score(&all_documents, &user_interests) {
        Ok(scores) => rank(&mut all_documents, &scores),
        Err(_) => {
            return Err(NotEnoughInteractions.into());
        }
    }
    let documents_by_interests = all_documents
        .iter()
        .enumerate()
        .map(|(rank, document)| (document.id.clone(), rank))
        .collect::<HashMap<_, _>>();

    let categories = state.storage.category().get(&user_id).await?;
    let mut documents_by_categories = all_documents
        .iter()
        .map(|document| {
            let weight = document
                .category
                .as_deref()
                .and_then(|category| categories.get(category).copied())
                .unwrap_or_default();
            (document.id.clone(), weight)
        })
        .collect_vec();
    documents_by_categories.sort_unstable_by(|(_, a), (_, b)| a.cmp(b).reverse());
    let documents_by_categories = documents_by_categories
        .into_iter()
        .enumerate()
        .map(|(rank, (document_id, _))| (document_id, rank))
        .collect::<HashMap<_, _>>();

    let weight = state.config.personalization.interest_category_bias;
    all_documents.sort_unstable_by(
        #[allow(clippy::cast_precision_loss)] // number of docs is small enough
        |a, b| {
            let weighted_a = documents_by_interests[&a.id] as f32 * weight
                + documents_by_categories[&a.id] as f32 * (1. - weight);
            let weighted_b = documents_by_interests[&b.id] as f32 * weight
                + documents_by_categories[&b.id] as f32 * (1. - weight);
            match nan_safe_f32_cmp(&weighted_a, &weighted_b) {
                Ordering::Equal if weight > 0.5 => {
                    documents_by_interests[&a.id].cmp(&documents_by_interests[&b.id])
                }
                Ordering::Equal if weight < 0.5 => {
                    documents_by_categories[&a.id].cmp(&documents_by_categories[&b.id])
                }
                ordering => ordering,
            }
        },
    );
    all_documents.truncate(document_count);

    Ok(Json(PersonalizedDocumentsResponse::new(all_documents)))
}

/// Represents response from personalized documents endpoint.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    pub(crate) documents: Vec<PersonalizedDocument>,
}

impl PersonalizedDocumentsResponse {
    pub(crate) fn new(documents: impl Into<Vec<PersonalizedDocument>>) -> Self {
        Self {
            documents: documents.into(),
        }
    }
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

/// Performs an approximate knn search for documents similar to the positive user interests.
async fn search_knn_documents(
    state: &AppState,
    user_id: &UserId,
    cois: &[PositiveCoi],
    document_count: usize,
) -> Result<Vec<PersonalizedDocument>, Error> {
    let horizon = state.coi.config().horizon();
    let max_cois = state
        .config
        .personalization
        .max_cois_for_knn
        .min(cois.len());
    let coi_weights = compute_coi_weights(cois, horizon);
    let cois = cois
        .iter()
        .zip(coi_weights)
        .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
        .collect_vec();

    let cois = &cois[0..max_cois];
    let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

    let excluded = state.storage.interaction().get(user_id).await?;

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
            let k_neighbors = (weight * document_count as f32).ceil() as usize;

            state
                .storage
                .document()
                .get_by_embedding(KnnSearchParams {
                    excluded: &excluded,
                    embedding: &coi.point,
                    k_neighbors,
                    num_candidates: document_count,
                })
                .await
        })
        .collect::<FuturesUnordered<_>>();

    let mut all_documents = HashMap::new();
    let mut errors = Vec::new();

    while let Some(result) = document_futures.next().await {
        match result {
            Ok(documents) => {
                // the same document can be returned with different elastic scores, hence the
                // documents are deduplicated and only the highest score is retained for each
                for document in documents {
                    all_documents
                        .entry(document.id.clone())
                        .and_modify(|PersonalizedDocument { score, .. }| {
                            if *score < document.score {
                                *score = document.score;
                            }
                        })
                        .or_insert(document);
                }
            }
            Err(error) => {
                error!("Error fetching documents: {error}");
                errors.push(error);
            }
        };
    }

    if all_documents.is_empty() && !errors.is_empty() {
        Err(InternalError::from_message("Fetching documents failed").into())
    } else {
        Ok(all_documents.into_values().collect_vec())
    }
}
