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
use tracing_subscriber::fmt::format::FmtSpan;
use warp::Filter;
use web_api::{api_routes, AppState, ElasticConfig, InitConfig};
use xayn_discovery_engine_ai::GenericError;

#[tokio::main]
async fn main() -> Result<(), GenericError> {
    let filter = env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,sqlx=warn,tracing=info,warp=debug".to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let pg_url = env::var("DE_POSTGRES_URL");
    let pg_url = pg_url.or_else(|_| dotenvy::var("DE_POSTGRES_URL"))?;
    let elastic_url =
        env::var("ELASTIC_URL").unwrap_or_else(|_| "http://localhost:9200".to_string());
    let elastic_index_name =
        env::var("ELASTIC_INDEX_NAME").unwrap_or_else(|_| "test_index".to_string());
    let elastic_user = env::var("ELASTIC_USER").unwrap_or_else(|_| "elastic".to_string());
    let elastic_password = env::var("ELASTIC_PASSWORD").unwrap_or_else(|_| "changeme".to_string());

    let port = env::var("DE_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    let ip_addr = env::var("DE_IP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0".to_string())
        .parse::<IpAddr>()?;

    let config = InitConfig {
        max_cois_for_knn: 5,
        default_documents_count: 10,
        pg_url,
        elastic: ElasticConfig {
            url: elastic_url,
            index_name: elastic_index_name,
            user: elastic_user,
            password: elastic_password,
        },
    };
    let state = AppState::init(config).await?;
    let routes = api_routes(state).with(warp::trace::request());

    warp::serve(routes).run((ip_addr, port)).await;
    Ok(())
}
