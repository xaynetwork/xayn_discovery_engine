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
    web::{self, Data, Json, ServiceConfig},
    HttpResponse,
    Responder,
};
use serde::Deserialize;

use crate::{
    config::{CommonConfig, Config},
    error::application::{Unimplemented, WithRequestIdExt},
    server::Application,
    Error,
};

pub struct Ingestion;

impl Application for Ingestion {
    type Config = IngestionConfig;
    type AppState = IngestionConfig;

    fn configure(config: &mut ServiceConfig) {
        let healthcheck =
            web::resource("/health").route(web::get().to(get_healthcheck.error_with_request_id()));
        let resource = web::resource("/documents")
            .route(web::post().to(new_documents.error_with_request_id()));

        config.service(healthcheck).service(resource);
    }
}

#[derive(Deserialize)]
pub struct IngestionConfig {
    #[serde(flatten)]
    common_config: CommonConfig,
}

impl Config for IngestionConfig {
    fn bind_address(&self) -> std::net::SocketAddr {
        self.common_config.bind_address()
    }

    fn log_file(&self) -> Option<&std::path::Path> {
        self.common_config.log_file()
    }
}

//FIXME use actual body
#[derive(Deserialize)]
struct NewDocuments {}

async fn get_healthcheck() -> Result<impl Responder, Error> {
    Ok(HttpResponse::Ok().finish())
}

async fn new_documents(
    _config: Data<IngestionConfig>,
    _new_documents: Json<NewDocuments>,
) -> Result<impl Responder, Error> {
    if true {
        Err(Unimplemented {
            functionality: "endpoint /documents",
        })?;
    }
    Ok("text body response")
}
