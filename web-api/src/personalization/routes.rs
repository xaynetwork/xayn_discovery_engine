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
    web::{self, Data, Json, Path, Query, ServiceConfig},
    HttpResponse,
    Responder,
};
use futures_util::{stream::FuturesUnordered, StreamExt};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tracing::error;
use xayn_ai_coi::{
    compute_coi_relevances,
    nan_safe_f32_cmp,
    system_time_now,
    utils::rank,
    CoiSystem,
    PositiveCoi,
};

use super::{AppState, PersonalizationConfig};
use crate::{
    error::{
        application::WithRequestIdExt,
        common::{BadRequest, InternalError, NotEnoughInteractions},
    },
    models::{DocumentId, PersonalizedDocument, UserId, UserInteractionType},
    storage::{self, KnnSearchParams},
    Error,
};

pub(super) fn configure_service(config: &mut ServiceConfig) {
    let scope = web::scope("/users/{user_id}")
        .service(
            web::resource("interactions")
                .route(web::patch().to(interactions.error_with_request_id())),
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

async fn interactions(
    state: Data<AppState>,
    user_id: Path<UserId>,
    Json(interactions): Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    update_interactions(
        &state.storage,
        &state.coi,
        &user_id,
        &interactions.documents,
    )
    .await?;

    Ok(HttpResponse::NoContent())
}

pub(crate) async fn update_interactions(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    interactions: &[UserInteractionData],
) -> Result<(), Error> {
    storage::Interaction::user_seen(storage, user_id).await?;

    #[allow(clippy::zero_sized_map_values)]
    let id_to_interaction_type = interactions
        .iter()
        .map(|interaction| (&interaction.document_id, interaction.interaction_type))
        .collect::<HashMap<_, _>>();

    let ids = interactions.iter().map(|i| &i.document_id).collect_vec();
    storage::Interaction::update_interactions(storage, user_id, &ids, |context| {
        match id_to_interaction_type[&context.document.id] {
            UserInteractionType::Positive => {
                for tag in &context.document.tags {
                    *context.tag_weight_diff
                            .get_mut(tag.as_str())
                            .unwrap(/*update_interactions assures all tags are given*/) += 1;
                }
                coi.log_positive_user_reaction(context.positive_cois, &context.document.embedding)
                    .clone()
            }
        }
    })
    .await?;

    Ok(())
}

/// Represents personalized documents query params.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PersonalizedDocumentsQuery {
    pub(crate) count: Option<usize>,
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

async fn personalized_documents(
    state: Data<AppState>,
    user_id: Path<UserId>,
    options: Query<PersonalizedDocumentsQuery>,
) -> Result<impl Responder, Error> {
    personalize_documents_by(
        &state.storage,
        &state.coi,
        &user_id,
        &state.config.personalization,
        PersonalizeBy::KnnSearch(options.document_count(&state.config.personalization)?),
    )
    .await
    .map(|documents| {
        Json(PersonalizedDocumentsResponse {
            documents: documents
                .into_iter()
                .map(|document| PersonalizedDocumentData {
                    id: document.id,
                    score: document.score,
                })
                .collect(),
        })
    })
}

#[derive(Debug, Serialize)]
struct PersonalizedDocumentData {
    id: DocumentId,
    score: f32,
}

/// Represents response from personalized documents endpoint.
#[derive(Debug, Serialize)]
struct PersonalizedDocumentsResponse {
    /// A list of documents personalized for a specific user.
    documents: Vec<PersonalizedDocumentData>,
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
    storage: &(impl storage::Document + storage::Interaction),
    user_id: &UserId,
    cois: &[PositiveCoi],
    horizon: Duration,
    max_cois: usize,
    count: usize,
) -> Result<Vec<PersonalizedDocument>, Error> {
    let coi_weights = compute_coi_weights(cois, horizon);
    let cois = cois
        .iter()
        .zip(coi_weights)
        .sorted_by(|(_, a_weight), (_, b_weight)| nan_safe_f32_cmp(b_weight, a_weight))
        .collect_vec();

    let cois = &cois[..max_cois.min(cois.len())];
    let weights_sum = cois.iter().map(|(_, w)| w).sum::<f32>();

    let excluded = storage::Interaction::get(storage, user_id).await?;

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
            let k_neighbors = (weight * count as f32).ceil() as usize;

            storage::Document::get_by_embedding(
                storage,
                KnnSearchParams {
                    excluded: &excluded,
                    embedding: &coi.point,
                    k_neighbors,
                    num_candidates: count,
                },
            )
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

pub(crate) enum PersonalizeBy<'a> {
    KnnSearch(usize),
    #[allow(dead_code)]
    Documents(&'a [&'a DocumentId]),
}

pub(crate) async fn personalize_documents_by(
    storage: &(impl storage::Document + storage::Interaction + storage::Interest + storage::Tag),
    coi: &CoiSystem,
    user_id: &UserId,
    personalization: &PersonalizationConfig,
    by: PersonalizeBy<'_>,
) -> Result<Vec<PersonalizedDocument>, Error> {
    storage::Interaction::user_seen(storage, user_id).await?;

    let user_interests = storage::Interest::get(storage, user_id).await?;
    if user_interests.is_empty() {
        return Err(NotEnoughInteractions.into());
    }

    let mut all_documents = match by {
        PersonalizeBy::KnnSearch(count) => {
            search_knn_documents(
                storage,
                user_id,
                &user_interests.positive,
                coi.config().horizon(),
                personalization.max_cois_for_knn,
                count,
            )
            .await?
        }
        PersonalizeBy::Documents(documents) => {
            storage::Document::get_by_ids(storage, documents).await?
        }
    };

    if let Ok(scores) = coi.score(&all_documents, &user_interests) {
        rank(&mut all_documents, &scores);
    } else {
        return Err(NotEnoughInteractions.into());
    }
    let documents_by_interests = all_documents
        .iter()
        .enumerate()
        .map(|(rank, document)| (document.id.clone(), rank))
        .collect::<HashMap<_, _>>();

    let tags = storage::Tag::get(storage, user_id).await?;
    let mut documents_by_tags = all_documents
        .iter()
        .map(|document| {
            let weight = document
                .tags
                .iter()
                .map(|tag| tags.get(tag))
                .sum::<Option<usize>>()
                .unwrap_or_default();
            (document.id.clone(), weight)
        })
        .collect_vec();
    documents_by_tags.sort_unstable_by(|(_, a), (_, b)| a.cmp(b).reverse());
    let documents_by_tags = documents_by_tags
        .into_iter()
        .enumerate()
        .map(|(rank, (document_id, _))| (document_id, rank))
        .collect::<HashMap<_, _>>();

    let weight = personalization.interest_tag_bias;
    all_documents.sort_unstable_by(
        #[allow(clippy::cast_precision_loss)] // number of docs is small enough
        |a, b| {
            let weighted_a = documents_by_interests[&a.id] as f32 * weight
                + documents_by_tags[&a.id] as f32 * (1. - weight);
            let weighted_b = documents_by_interests[&b.id] as f32 * weight
                + documents_by_tags[&b.id] as f32 * (1. - weight);
            match nan_safe_f32_cmp(&weighted_a, &weighted_b) {
                Ordering::Equal if weight > 0.5 => {
                    documents_by_interests[&a.id].cmp(&documents_by_interests[&b.id])
                }
                Ordering::Equal if weight < 0.5 => {
                    documents_by_tags[&a.id].cmp(&documents_by_tags[&b.id])
                }
                ordering => ordering,
            }
        },
    );
    if let PersonalizeBy::KnnSearch(count) = by {
        all_documents.truncate(count);
    }

    Ok(all_documents)
}
