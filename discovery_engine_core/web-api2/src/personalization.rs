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

use actix_web::{web::{ServiceConfig, Data, Path, Json}, patch, get, Responder};

pub fn mount_personalization(config: &mut ServiceConfig) {
    config.service(update_interactions)
        .service(personalized_documents);
}

#[patch("/users/{user_id}/interactions")]
async fn update_interactions(app_state: Data<AppState>, user_id: Path<UserId>, interactions: Json<UpdateInteractions>) -> impl Responder {
    "foo"
}

#[get("/users/{user_id}/personalized_documents")]
async fn personalized_documents(app_state: Data<AppState>, user_id: Path<UserId>) -> impl Responder {
    "foo"
}
