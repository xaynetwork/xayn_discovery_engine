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
    get,
    patch,
    web::{Data, Json, Path, ServiceConfig},
    Responder,
};
use serde::Deserialize;

use crate::{
    config::Config,
    error::application::Unimplemented,
    server::{default_bind_address, Application},
    Error,
};

pub struct Personalization;

impl Application for Personalization {
    type Config = PersonalizationConfig;

    fn configure(config: &mut ServiceConfig) {
        config
            .service(update_interactions)
            .service(personalized_documents);
    }
}

#[derive(Deserialize)]
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
#[derive(Deserialize)]
struct UpdateInteractions {}

#[patch("/users/{user_id}/interactions")]
async fn update_interactions(
    _config: Data<PersonalizationConfig>,
    _user_id: Path<UserId>,
    _interactions: Json<UpdateInteractions>,
) -> Result<impl Responder, Error> {
    if true {
        Err(Unimplemented {
            functionality: "/users/{user_id}/interactions",
        })?;
    }
    Ok("text body response")
}

#[get("/users/{user_id}/personalized_documents")]
async fn personalized_documents(
    _config: Data<PersonalizationConfig>,
    _user_id: Path<UserId>,
) -> Result<impl Responder, Error> {
    if true {
        Err(Unimplemented {
            functionality: "/users/{user_id}/personalized_documents",
        })?;
    }
    Ok("text body response")
}
