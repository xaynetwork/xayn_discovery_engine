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
    web::{self, ServiceConfig},
    Responder,
};
use interactions::interactions;
use recommendations::{recommendations, user_recommendations};
use semantic_search::semantic_search;

use super::{PersonalizationConfig, SemanticSearchConfig};
use crate::utils::deprecate;

mod interactions;
mod recommendations;
mod semantic_search;

pub(crate) fn configure_service(config: &mut ServiceConfig) {
    let users = web::scope("/users/{user_id}")
        .service(web::resource("interactions").route(web::patch().to(interactions)))
        .service(web::resource("recommendations").route(web::post().to(user_recommendations)))
        .service(
            web::resource("personalized_documents")
                .route(web::post().to(deprecate!(user_recommendations(
                    state, user_id, body, params, storage,
                ))))
                // this route is deprecated and will be removed in the future
                .route(web::get().to(deprecate!(user_recommendations(
                    state, user_id, body, params, storage,
                )))),
        );
    let semantic_search = web::resource("/semantic_search").route(web::post().to(semantic_search));
    let recommendations_service =
        web::resource("/recommendations").route(web::post().to(recommendations));

    config
        .service(users)
        .service(semantic_search)
        .service(recommendations_service);
}
