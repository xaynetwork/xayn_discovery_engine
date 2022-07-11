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
use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr},
};
use uuid::Uuid;
use warp::{hyper::StatusCode, Filter};

#[tokio::main]
async fn main() {
    // GET /ranked-documents/:userId
    let ranked_documents = warp::path("ranked-documents")
        .and(warp::path::param::<Uuid>())
        .and(warp::get())
        .and_then(handle_ranked_documents);

    // POST /user-interaction
    let user_interaction = warp::path("user-interaction")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 16).and(warp::body::json()))
        .and_then(handle_user_interaction);

    // DELETE /clean-state
    let clean_state = warp::path("clean-state")
        .and(warp::header::exact("authorization", "Bearer token"))
        .and(warp::delete())
        .and_then(handle_clean_state);

    let routes = ranked_documents.or(user_interaction).or(clean_state);

    // TODO: TY-3012
    let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let port = 3000;

    warp::serve(routes).run((ip_addr, port)).await;
}

#[derive(Serialize, Deserialize)]
struct UserInteractionDto {
    user_id: Uuid,
    document_id: Uuid,
}

// TODO: TY-3013
async fn handle_ranked_documents(_user_id: Uuid) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// TODO: TY-3014
async fn handle_user_interaction(
    _body: UserInteractionDto,
) -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

// TODO: TY-3015
async fn handle_clean_state() -> Result<impl warp::Reply, Infallible> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}
