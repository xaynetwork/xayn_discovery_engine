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

use std::net::SocketAddr;

use actix_web::{
    web::{self, Data, Json, Path, ServiceConfig},
    Responder,
};
use serde::Deserialize;

use crate::{
    config::Config,
    error::application::{Unimplemented, WithRequestIdExt},
    server::{default_bind_address, Application},
    Error,
};

pub struct Personalization;

impl Application for Personalization {
    type Config = PersonalizationConfig;
    type AppState = PersonalizationConfig;

    fn configure(config: &mut ServiceConfig) {
        let scope = web::scope("/users/{user_id}")
            .service(
                web::resource("interactions")
                    .route(web::post().to(update_interactions.error_with_request_id())),
            )
            .service(
                web::resource("personalized_documents")
                    .route(web::get().to(personalized_documents.error_with_request_id())),
            );

        config.service(scope);
    }
}

#[derive(Deserialize, Debug)]
pub struct PersonalizationConfig {
    #[serde(default = "default_bind_address")]
    bind_to: SocketAddr,
}

impl Config for PersonalizationConfig {
    fn bind_address(&self) -> std::net::SocketAddr {
        self.bind_to
    }
}

//FIXME use actual UserId
type UserId = String;

//FIXME use actual body
#[derive(Deserialize, Debug)]
struct UpdateInteractions {}

async fn update_interactions(
    config: Data<PersonalizationConfig>,
    user_id: Path<UserId>,
    interactions: Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    dbg!((config, user_id, interactions));
    if true {
        Err(Unimplemented {
            functionality: "/users/{user_id}/interactions",
        })?;
    }
    Ok("text body response")
}

async fn personalized_documents(
    config: Data<PersonalizationConfig>,
    user_id: Path<UserId>,
) -> Result<impl Responder, Error> {
    dbg!((config, user_id));
    if true {
        Err(Unimplemented {
            functionality: "/users/{user_id}/personalized_documents",
        })?;
    }
    Ok("text body response")
}
