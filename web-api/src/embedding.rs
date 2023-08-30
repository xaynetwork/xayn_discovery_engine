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

use aws_config::retry::RetryConfig;
use aws_sdk_sagemakerruntime::{config::Region, primitives::Blob, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use xayn_ai_bert::{AvgEmbedder, Config as EmbedderConfig, NormalizedEmbedding};

use crate::{app::SetupError, error::common::InternalError, utils::RelativePathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Config {
    Pipeline(Pipeline),
    Sagemaker(Sagemaker),
}

impl Default for Config {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pipeline {
    #[serde(deserialize_with = "RelativePathBuf::deserialize_string")]
    pub(crate) directory: RelativePathBuf,
    #[serde(deserialize_with = "RelativePathBuf::deserialize_string")]
    pub(crate) runtime: RelativePathBuf,
    pub(crate) token_size: usize,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            directory: "assets".into(),
            runtime: "assets".into(),
            token_size: 250,
        }
    }
}

impl Pipeline {
    fn load(&self) -> Result<Embedder, SetupError> {
        let config = EmbedderConfig::new(self.directory.relative(), self.runtime.relative())?
            .with_token_size(self.token_size)?
            .with_pooler();
        config.validate()?;
        let embedder = config.build()?;

        Ok(Embedder::Pipeline(embedder))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sagemaker {
    pub(crate) endpoint: String,
    pub(crate) embedding_size: usize,
    pub(crate) target_model: Option<String>,
    pub(crate) retry_max_attempts: Option<u32>,
    pub(crate) aws_region: Option<String>,
    pub(crate) aws_profile: Option<String>,
}

impl Sagemaker {
    async fn load(&self) -> Result<Embedder, SetupError> {
        let mut config_loader = aws_config::from_env();

        if let Some(region) = &self.aws_region {
            config_loader = config_loader.region(Region::new(region.clone()));
        }
        if let Some(profile) = &self.aws_profile {
            config_loader = config_loader.profile_name(profile);
        }

        config_loader = config_loader.retry_config(
            RetryConfig::standard()
                .with_max_attempts(1 + self.retry_max_attempts.unwrap_or_default()),
        );

        let sdk_config = config_loader.load().await;
        let client = Client::new(&sdk_config);

        Ok(Embedder::Sagemaker {
            client,
            embedding_size: self.embedding_size,
            endpoint: self.endpoint.clone(),
            target_model: self.target_model.clone(),
        })
    }
}

pub(crate) enum Embedder {
    Pipeline(AvgEmbedder),
    Sagemaker {
        client: Client,
        endpoint: String,
        embedding_size: usize,
        target_model: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct SagemakerResponse {
    embeddings: Vec<NormalizedEmbedding>,
}

impl Embedder {
    pub(crate) async fn load(config: &Config) -> Result<Self, SetupError> {
        match config {
            Config::Pipeline(config) => config.load(),
            Config::Sagemaker(config) => config.load().await,
        }
    }

    pub(crate) async fn run(&self, sequence: &str) -> Result<NormalizedEmbedding, InternalError> {
        match self {
            Embedder::Pipeline(embedder) => embedder
                .run(sequence)
                .map_err(InternalError::from_std)?
                .normalize()
                .map_err(InternalError::from_std),
            Embedder::Sagemaker {
                client,
                endpoint,
                target_model,
                ..
            } => Self::run_sagemaker(client, endpoint, target_model.as_deref(), sequence).await,
        }
    }

    async fn run_sagemaker(
        client: &Client,
        endpoint: &str,
        target_model: Option<&str>,
        sequence: &str,
    ) -> Result<NormalizedEmbedding, InternalError> {
        let input = json!({
            "inputs": [sequence],
        });
        let mut request = client
            .invoke_endpoint()
            .endpoint_name(endpoint)
            .content_type("application/json")
            .body(Blob::new(input.to_string()));

        if let Some(target_model) = target_model {
            request = request.target_model(target_model);
        };

        let response = request.send().await.map_err(InternalError::from_std)?;

        let Some(body) = response.body() else {
            return Err(InternalError::from_message(
                "Received sagemaker response without body.",
            ));
        };

        let mut embeddings = serde_json::from_slice::<SagemakerResponse>(body.as_ref())
            .map_err(InternalError::from_std)?
            .embeddings;

        if embeddings.len() == 1 {
            Ok(
                embeddings.pop().unwrap(/* safe because we check that embeddings contains one item */),
            )
        } else {
            Err(InternalError::from_message(format!(
                "Unexpected sagemaker response. Expected 1 embedding, got {}",
                embeddings.len()
            )))
        }
    }

    pub(crate) fn embedding_size(&self) -> usize {
        match self {
            Embedder::Pipeline(embedder) => embedder.embedding_size(),
            Embedder::Sagemaker { embedding_size, .. } => *embedding_size,
        }
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::{ort, xaynia};

    use super::*;

    #[tokio::test]
    async fn test_embedder() {
        let config = Config::Pipeline(Pipeline {
            directory: xaynia().unwrap().into(),
            runtime: ort().unwrap().into(),
            ..Pipeline::default()
        });
        let embedder = Embedder::load(&config).await.unwrap();
        embedder.run("test").await.unwrap();
    }
}
