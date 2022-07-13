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

//! Web service that uses Xayn Discovery Engine.

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

use serde::{Deserialize, Serialize};
use std::{convert::Infallible, env, net::IpAddr};
use uuid::Uuid;
use warp::{hyper::StatusCode, Filter};

#[tokio::main]
async fn main() {
    // TODO: TY-3011 - add filepath env var for documents data json file
    let port = env::var("DE_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap();
    let ip_addr = env::var("DE_IP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0".to_string())
        .parse::<IpAddr>()
        .unwrap();

    // PATH /user/:user_id
    let user_route = warp::path("user").and(warp::path::param::<Uuid>());

    // GET /user/:user_id/documents
    let ranked_documents = user_route
        .and(warp::path("documents"))
        .and(warp::get())
        .and_then(handle_ranked_documents);

    // POST /user/:user_id/interaction
    let user_interaction = user_route
        .and(warp::path("interaction"))
        .and(warp::post())
        .and(warp::body::content_length_limit(1024))
        .and(warp::body::json())
        .and_then(handle_user_interaction);

    // DELETE /internal-state
    let clean_state = warp::path("internal-state")
        .and(warp::delete())
        .and_then(handle_clean_state);

    let routes = ranked_documents.or(user_interaction).or(clean_state);

    warp::serve(routes).run((ip_addr, port)).await;
}

// TODO: TY-3013
#[allow(clippy::unused_async)]
async fn handle_ranked_documents(_user_id: Uuid) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

#[derive(Serialize, Deserialize)]
struct UserInteractionDto {
    document_id: Uuid,
}

// TODO: TY-3014
#[allow(clippy::unused_async)]
async fn handle_user_interaction(
    _user_id: Uuid,
    _body: UserInteractionDto,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// TODO: TY-3015
#[allow(clippy::unused_async)]
async fn handle_clean_state() -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}
