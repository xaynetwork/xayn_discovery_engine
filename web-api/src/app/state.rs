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

use std::sync::Arc;

use actix_web::{
    dev::{Payload, ServiceFactory, ServiceRequest},
    web::Data,
    App,
    FromRequest,
    HttpRequest,
};
use derive_more::{AsRef, Deref};
use futures_util::future::{ready, Ready};

use crate::{
    app::{Application, SetupError},
    error::common::InternalError,
    middleware::request_context::RequestContext,
    storage::{Storage, StorageBuilder},
    Error,
};

#[derive(AsRef, Deref)]
pub(crate) struct AppState<A>
where
    A: Application,
{
    #[as_ref(forward)]
    pub(crate) config: A::Config,
    #[deref]
    pub(crate) extension: A::Extension,
    storage_builder: Arc<StorageBuilder>,
}

impl<A> AppState<A>
where
    A: Application,
{
    pub(super) fn attach_to<T>(self: Arc<Self>, app: App<T>) -> App<T>
    where
        T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
    {
        app.app_data(self.storage_builder.clone())
            .app_data(Data::from(self))
            .configure(A::configure_service)
    }

    pub(super) async fn create(config: A::Config) -> Result<Self, SetupError> {
        let extension = A::create_extension(&config)?;
        let storage_builder = Arc::new(Storage::builder(config.as_ref()).await?);
        Ok(Self {
            config,
            extension,
            storage_builder,
        })
    }

    pub(super) async fn close(self: Arc<Self>) {
        self.storage_builder.close().await;
    }
}

/// Extract tenant specific state.
///
/// For now this only extracts storage.
pub(crate) struct TenantState(pub(crate) Storage);

impl FromRequest for TenantState {
    type Error = Error;

    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(extract_tenant_state(request))
    }
}

fn extract_tenant_state(request: &HttpRequest) -> Result<TenantState, Error> {
    RequestContext::try_extract_from_request(request, |ctx| {
        let storage = request
            .app_data::<Arc<StorageBuilder>>()
            .ok_or_else(|| InternalError::from_message("Arc<StorageBuilder> missing"))?
            .build_for(&ctx.tenant_id)?;
        Ok(TenantState(storage))
    })
    .map_err(InternalError::from_std)?
}
