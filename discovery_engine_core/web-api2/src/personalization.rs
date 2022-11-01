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

mod routes;

use actix_web::web::ServiceConfig;
use derive_more::AsRef;
use serde::{Deserialize, Serialize};
use xayn_discovery_engine_ai::{CoiConfig, CoiSystem};

use crate::server::{self, Application};

pub struct Personalization;

impl Application for Personalization {
    type ConfigExtension = ConfigExtension;
    type AppStateExtension = AppStateExtension;

    fn configure_service(config: &mut ServiceConfig) {
        routes::configure_service(config);
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
