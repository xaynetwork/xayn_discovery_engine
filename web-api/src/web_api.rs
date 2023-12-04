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

use actix_web::web::ServiceConfig;
use async_trait::async_trait;

use crate::{app::Application, config::WebApiConfig};

pub struct WebApi;

#[async_trait]
impl Application for WebApi {
    const NAME: &'static str = "XAYN_WEB_API";

    type Config = WebApiConfig;

    fn configure_service(config: &mut ServiceConfig) {
        crate::backoffice::routes::configure_service(config);
        crate::frontoffice::routes::configure_service(config);
    }

    fn configure_ops_service(config: &mut ServiceConfig) {
        crate::backoffice::routes::configure_ops_service(config);
    }
}
