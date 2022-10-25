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

use actix_web::{
    web::{self, Data, Json, Path, ServiceConfig},
    Responder,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::application::{Unimplemented, WithRequestIdExt},
    server::{self, Application},
    Error,
};

pub struct Personalization;

impl Application for Personalization {
    type ConfigExtension = ConfigExtension;

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

type Config = server::Config<<Personalization as Application>::ConfigExtension>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigExtension {}

//FIXME use actual UserId
type UserId = String;

//FIXME use actual body
#[derive(Deserialize, Debug)]
struct UpdateInteractions {}

async fn update_interactions(
    config: Data<Config>,
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
    config: Data<Config>,
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
