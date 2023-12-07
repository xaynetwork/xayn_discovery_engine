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
    dev::Payload,
    web::{Data, ServiceConfig},
    FromRequest, HttpRequest,
};
use derive_more::AsRef;
use futures_util::{future::BoxFuture, FutureExt};
use xayn_ai_coi::CoiSystem;
use xayn_snippet_extractor::pool::SnippetExtractorPool;
use xayn_web_api_db_ctrl::Silo;
use xayn_web_api_shared::request::TenantId;

use crate::{
    app::SetupError,
    config::Config,
    embedding::{Embedder, Models},
    error::common::InternalError,
    extractor::TextExtractor,
    middleware::request_context::RequestContext,
    storage::{initialize_silo, Storage, StorageBuilder},
    Error,
};

#[derive(AsRef)]
pub(crate) struct AppState {
    #[as_ref(forward)]
    pub(crate) config: Config,
    pub(crate) models: Models,
    pub(crate) extractor: TextExtractor,
    pub(crate) snippet_extractor: SnippetExtractorPool,
    pub(crate) coi: CoiSystem,
    storage_builder: Arc<StorageBuilder>,
    silo: Arc<Silo>,
}

impl AppState {
    pub(super) fn attach_to(self: Arc<Self>, service: &mut ServiceConfig) {
        service
            .app_data(self.storage_builder.clone())
            .app_data(Data::from(self.silo.clone()))
            .app_data(self.models.clone())
            .app_data(Data::from(self));
    }

    pub(super) async fn create(config: Config) -> Result<Self, SetupError> {
        let extractor = TextExtractor::new(config.as_ref())?;
        let models = Models::load(config.as_ref(), config.as_ref()).await?;
        let (silo, legacy_tenant) =
            initialize_silo(config.as_ref(), config.as_ref(), models.embedding_sizes()).await?;
        let storage_builder = Arc::new(Storage::builder(config.as_ref(), legacy_tenant).await?);
        let snippet_extractor = SnippetExtractorPool::new(config.as_ref())?;
        Ok(Self {
            coi: config.coi.clone().build(),
            config,
            models,
            extractor,
            snippet_extractor,
            storage_builder,
            silo: Arc::new(silo),
        })
    }

    pub(super) async fn close(self: Arc<Self>) {
        self.storage_builder.close().await;
    }

    pub(crate) fn legacy_tenant(&self) -> Option<&TenantId> {
        self.storage_builder.legacy_tenant()
    }
}

/// Extract tenant specific state.
///
/// For now this only extracts storage.
pub(crate) struct TenantState(pub(crate) Storage, pub(crate) Arc<Embedder>);

impl FromRequest for TenantState {
    type Error = Error;

    type Future = BoxFuture<'static, Result<TenantState, Error>>;

    fn from_request(request: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let result = RequestContext::try_extract_from_request(request, |ctx| {
            let builder = request.app_data::<Arc<StorageBuilder>>()?.clone();
            let models = request.app_data::<Models>()?.clone();
            let tenant_id = ctx.tenant_id.clone();
            Some((builder, models, tenant_id))
        });

        async move {
            match result {
                Ok(Some((builder, models, tenant_id))) => {
                    let storage = builder.build_for(tenant_id).await?;
                    let model = &storage.tenant().model;
                    if let Some(embedder) = models.get(model) {
                        Ok(TenantState(storage, embedder.clone()))
                    } else {
                        Err(InternalError::from_message(format!(
                            "deployment doesn't support tenants model: {model}"
                        ))
                        .into())
                    }
                }
                Ok(None) => Err(InternalError::from_message("Arc<StorageBuilder> missing").into()),
                Err(error) => Err(InternalError::from_std(error).into()),
            }
        }
        .boxed()
    }
}
