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
use std::{env, net::IpAddr};

use xayn_discovery_engine_ai::GenericError;

use db::{init_db, InitConfig};
use routes::api_routes;
use storage::UserState;

mod db;
mod handlers;
mod models;
mod routes;
mod storage;

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let pg_url = env::var("DE_POSTGRES_URL");

    let path = env::current_dir().unwrap();
    let smbert_vocab = path.join(dotenvy::var("DE_SMBERT_VOCAB")?);
    let smbert_model = path.join(dotenvy::var("DE_SMBERT_MODEL")?);
    let data_store = path.join(dotenvy::var("DE_DATA_PATH")?);
    let pg_url = pg_url.or_else(|_| dotenvy::var("DE_POSTGRES_URL"))?;

    let port = env::var("DE_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    let ip_addr = env::var("DE_IP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0".to_string())
        .parse::<IpAddr>()?;

    let user_state = UserState::connect(&pg_url).await?;
    user_state.init_database().await?;

    let config = InitConfig {
        smbert_vocab,
        smbert_model,
        data_store,
        user_state,
    };
    let db = init_db(&config)?;
    let routes = api_routes(db);

    warp::serve(routes).run((ip_addr, port)).await;
    Ok(())
}
