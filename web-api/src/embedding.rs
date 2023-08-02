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

use std::str;

use aws_sdk_sagemakerruntime::{config::Region, primitives::Blob, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use xayn_ai_bert::NormalizedEmbedding;

use crate::{app::SetupError, error::common::InternalError};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub(crate) token_size: usize,
    pub(crate) sagemaker_endpoint_name: String,
    pub(crate) aws_region: Option<String>,
    pub(crate) aws_profile: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token_size: 250,
            sagemaker_endpoint_name: String::new(),
            aws_region: None,
            aws_profile: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Response {
    embeddings: Vec<NormalizedEmbedding>,
}

pub(crate) struct Embedder {
    embedding_dim: usize,
    client: Client,
    sagemaker_endpoint_name: String,
}

impl Embedder {
    pub(crate) async fn load(config: &Config) -> Result<Self, SetupError> {
        let mut config_loader = aws_config::from_env();

        if let Some(region) = &config.aws_region {
            config_loader = config_loader.region(Region::new(region.clone()));
        }
        if let Some(profile) = &config.aws_profile {
            config_loader = config_loader.profile_name(profile.clone());
        }

        let sdk_config = config_loader.load().await;
        let client = Client::new(&sdk_config);

        Ok(Self {
            embedding_dim: 384,
            client,
            sagemaker_endpoint_name: config.sagemaker_endpoint_name.clone(),
        })
    }

    pub(crate) async fn run(&self, sequence: &str) -> Result<NormalizedEmbedding, InternalError> {
        let input = json!({
            "inputs": [sequence],
        });
        let res = self
            .client
            .invoke_endpoint()
            .endpoint_name(self.sagemaker_endpoint_name.clone())
            .content_type("application/json")
            .body(Blob::new(input.to_string()))
            .send()
            .await
            .map_err(|e| {
                InternalError::from_message(format!(
                    "Failed to request sagemaker endpoint. Error: {e}"
                ))
            })?;
        let body = res.body().ok_or(InternalError::from_message(
            "Received sagemaker response without body.",
        ))?;
        let mut embeddings: Response = serde_json::from_slice(body.as_ref()).map_err(|e| {
            InternalError::from_message(format!(
                "Failed to deserialize sagemaker response body. Error: {e}"
            ))
        })?;
        embeddings
            .embeddings
            .pop()
            .ok_or(InternalError::from_message(
                "Missing embedding in sagemaker response.",
            ))
    }

    pub(crate) fn embedding_size(&self) -> usize {
        // self.embedder.embedding_size()
        self.embedding_dim
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::xaynia;

    use super::*;

    #[tokio::test]
    async fn test_embedder() {
        let config = Config {
            token_size: 250,
            sagemaker_endpoint_name: "e5-small-v2-endpoint".to_string(),
            aws_profile: Some("AdministratorAccess-917039226361".to_string()),
            aws_region: Some("eu-central-1".to_string()),
        };
        let embedder = Embedder::load(&config).await.unwrap();
        embedder.run("test").await.unwrap();
    }
}
