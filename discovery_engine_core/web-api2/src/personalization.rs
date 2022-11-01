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
use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use xayn_discovery_engine_ai::{CoiConfig, CoiSystem};

use crate::{
    error::application::{Unimplemented, WithRequestIdExt},
    server::{self, Application},
    Error,
};

pub struct Personalization;

impl Application for Personalization {
    type ConfigExtension = ConfigExtension;
    type AppStateExtension = AppStateExtension;

    fn configure_service(config: &mut ServiceConfig) {
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

    fn create_app_state_extension(
        config: &server::Config<Self::ConfigExtension>,
    ) -> Result<Self::AppStateExtension, server::SetupError> {
        Ok(AppStateExtension {
            coi: config.extension.coi.clone().build(),
        })
    }
}

type Config = server::Config<<Personalization as Application>::ConfigExtension>;

#[derive(AsRef, Debug, Default, Deserialize, Serialize)]
pub struct ConfigExtension {
    #[allow(dead_code)]
    #[as_ref]
    #[serde(default)]
    pub(crate) coi: CoiConfig,
}

#[derive(AsRef)]
pub struct AppStateExtension {
    #[as_ref]
    #[allow(dead_code)]
    pub(crate) coi: CoiSystem,
}

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
