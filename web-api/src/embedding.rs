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

// use xayn_ai_bert::{AvgEmbedder, Config as EmbedderConfig, NormalizedEmbedding};
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sagemakerruntime::{primitives::Blob, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing_subscriber::fmt::format;
use xayn_ai_bert::{Embedding1, NormalizedEmbedding};

use crate::{app::SetupError, error::common::InternalError};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub(crate) token_size: usize,
    pub(crate) sagemaker_endpoint_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token_size: 250,
            sagemaker_endpoint_name: String::new(),
        }
    }
}

pub(crate) struct Embedder {
    // embedder: AvgEmbedder,
    embedding_dim: usize,
    client: Client,
    sagemaker_endpoint_name: String,
}

impl Embedder {
    pub(crate) async fn load(config: &Config) -> Result<Self, SetupError> {
        // let config = EmbedderConfig::new(config.directory.relative())?
        //     .with_token_size(config.token_size)?
        //     .with_pooler();
        // config.validate()?;
        // let embedder = config.build()?;

        let region_provider = RegionProviderChain::default_provider().or_else("eu-central-1");
        let sdk_config = aws_config::from_env()
            .credentials_provider(
                aws_config::profile::ProfileFileCredentialsProvider::builder()
                    // If you need a specific profile, uncomment this line:
                    .profile_name("AdministratorAccess-917039226361")
                    .build(),
            )
            .region(region_provider)
            .load()
            .await;
        // let sdk_config = aws_config::from_env().region(region_provider).load().await;
        let client = Client::new(&sdk_config);

        Ok(Self {
            embedding_dim: 384,
            client: client,
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
            .unwrap();
        let body = res.body().ok_or(InternalError::from_message(
            "Received sagemaker response without body.",
        ))?;
        let mut embeddings: Vec<Vec<Vec<Embedding1>>> = serde_json::from_slice(body.as_ref())
            .map_err(|e| {
                InternalError::from_message(format!("Failed to deserialize sagemaker response body. Error: {}", e))
            })?;
        embeddings
            .pop()
            .and_then(|mut inner| inner.pop())
            .and_then(|mut inner| inner.pop())
            .and_then(|e| e.normalize().ok())
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
        };
        let embedder = Embedder::load(&config).await.unwrap();
        embedder.run("test").await.unwrap();
    }
}
