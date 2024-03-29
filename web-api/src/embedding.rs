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

use std::{collections::HashMap, sync::Arc};

use anyhow::bail;
use aws_config::retry::RetryConfig;
use aws_sdk_sagemakerruntime::{config::Region, primitives::Blob};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;
use xayn_ai_bert::{AvgEmbedder, Config as EmbedderConfig, Embedding1, NormalizedEmbedding};
use xayn_web_api_shared::serde::serialize_redacted;

use crate::{app::SetupError, error::common::InternalError, utils::RelativePathBuf};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct MultiConfig(HashMap<String, Config>);

impl MultiConfig {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn inject_default(&mut self, config: Config) -> Result<(), SetupError> {
        if self.0.contains_key("default") {
            bail!("default embedder is configured twice once explicit in \"models\" and once implicit through \"embedding\"");
        }
        self.0.insert("default".to_owned(), config);
        Ok(())
    }

    pub(crate) fn has_default_model(&self) -> bool {
        self.0.contains_key("default")
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Config {
    Pipeline(Pipeline),
    Sagemaker(Sagemaker),
    OpenAi(OpenAi),
}

impl Default for Config {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub(crate) struct Prefix {
    /// Prefix prepended to search queries when embedding them.
    pub(crate) query: String,
    /// Prefix prepended to content when creating embedding for it.
    pub(crate) snippet: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Pipeline {
    #[serde(deserialize_with = "RelativePathBuf::deserialize_string")]
    pub(crate) directory: RelativePathBuf,
    #[serde(deserialize_with = "RelativePathBuf::deserialize_string")]
    pub(crate) runtime: RelativePathBuf,
    pub(crate) token_size: usize,
    pub(crate) prefix: Prefix,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            directory: "assets".into(),
            runtime: "assets".into(),
            token_size: 250,
            prefix: Prefix::default(),
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

        Ok(Embedder {
            prefix: self.prefix.clone(),
            inner: InnerEmbedder::Pipeline(embedder),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct Sagemaker {
    pub(crate) endpoint: String,
    pub(crate) embedding_size: usize,
    pub(crate) target_model: Option<String>,
    pub(crate) retry_max_attempts: Option<u32>,
    pub(crate) aws_region: Option<String>,
    pub(crate) aws_profile: Option<String>,
    #[serde(default)]
    pub(crate) prefix: Prefix,
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
        let client = aws_sdk_sagemakerruntime::Client::new(&sdk_config);

        Ok(Embedder {
            prefix: self.prefix.clone(),
            inner: InnerEmbedder::Sagemaker {
                client,
                embedding_size: self.embedding_size,
                endpoint: self.endpoint.clone(),
                target_model: self.target_model.clone(),
            },
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct OpenAi {
    pub(crate) url: String,
    #[serde(serialize_with = "serialize_redacted")]
    pub(crate) api_key: Secret<String>,
    pub(crate) embedding_size: usize,
    #[serde(default)]
    pub(crate) prefix: Prefix,
}

impl OpenAi {
    fn load(&self) -> Result<Embedder, SetupError> {
        use reqwest::{
            header::{HeaderMap, HeaderValue},
            ClientBuilder,
        };

        let headers = {
            let mut api_key_value = HeaderValue::from_str(self.api_key.expose_secret())?;
            api_key_value.set_sensitive(true);

            let mut headers = HeaderMap::with_capacity(1);
            headers.insert("api-key", api_key_value);

            headers
        };

        let client = ClientBuilder::new().default_headers(headers).build()?;
        let url = self.url.parse()?;

        Ok(Embedder {
            prefix: self.prefix.clone(),
            inner: InnerEmbedder::OpenAi {
                client,
                url,
                embedding_size: self.embedding_size,
            },
        })
    }
}

#[derive(Clone)]
pub(crate) struct Models(Arc<HashMap<String, Arc<Embedder>>>);

impl Models {
    pub(crate) async fn load(
        config: &MultiConfig,
        inject_default: &Option<Config>,
    ) -> Result<Self, SetupError> {
        if config.0.contains_key("default") && inject_default.is_some() {
            bail!("model \"default\" is declared twice once explicit in \"models\" and once implicit through the \"embedding\" config");
        }
        let mut embedders = HashMap::new();
        if let Some(default) = inject_default.as_ref() {
            let embedder = Embedder::load(default).await?;
            embedders.insert("default".to_owned(), Arc::new(embedder));
        };
        for (name, config) in &config.0 {
            let embedder = Embedder::load(config).await?;
            embedders.insert(name.clone(), Arc::new(embedder));
        }
        Ok(Self(Arc::new(embedders)))
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Arc<Embedder>> {
        self.0.get(name)
    }

    pub(crate) fn embedding_sizes(&self) -> HashMap<String, usize> {
        self.0
            .iter()
            .map(|(name, embedder)| (name.clone(), embedder.embedding_size()))
            .collect()
    }
}

pub(crate) struct Embedder {
    prefix: Prefix,
    inner: InnerEmbedder,
}

enum InnerEmbedder {
    Pipeline(AvgEmbedder),
    Sagemaker {
        client: aws_sdk_sagemakerruntime::Client,
        endpoint: String,
        embedding_size: usize,
        target_model: Option<String>,
    },
    OpenAi {
        client: reqwest::Client,
        url: Url,
        embedding_size: usize,
    },
}

#[derive(Debug, Deserialize, Serialize)]
struct SagemakerResponse {
    embeddings: Vec<NormalizedEmbedding>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    data: Vec<OpenAiResponseData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseData {
    embedding: Embedding1,
}

#[derive(Copy, Clone)]
pub(crate) enum EmbeddingKind {
    Query,
    Content,
}

impl Embedder {
    pub(crate) async fn load(config: &Config) -> Result<Self, SetupError> {
        match config {
            Config::Pipeline(config) => config.load(),
            Config::Sagemaker(config) => config.load().await,
            Config::OpenAi(config) => config.load(),
        }
    }

    pub(crate) async fn run(
        &self,
        kind: EmbeddingKind,
        sequence: &str,
    ) -> Result<NormalizedEmbedding, InternalError> {
        let prefix = match (kind, &self.prefix) {
            (EmbeddingKind::Query, Prefix { query, .. }) => query,
            (
                EmbeddingKind::Content,
                Prefix {
                    snippet: content, ..
                },
            ) => content,
        };
        let sequence = format!("{prefix}{sequence}");

        match &self.inner {
            InnerEmbedder::Pipeline(embedder) => embedder
                .run(sequence)
                .map_err(InternalError::from_std)?
                .normalize()
                .map_err(InternalError::from_std),
            InnerEmbedder::Sagemaker {
                client,
                endpoint,
                target_model,
                ..
            } => Self::run_sagemaker(client, endpoint, target_model.as_deref(), &sequence).await,
            InnerEmbedder::OpenAi { client, url, .. } => {
                Self::run_openai(client, url, &sequence).await
            }
        }
    }

    async fn run_sagemaker(
        client: &aws_sdk_sagemakerruntime::Client,
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

    async fn run_openai(
        client: &reqwest::Client,
        url: &Url,
        sequence: &str,
    ) -> Result<NormalizedEmbedding, InternalError> {
        let input = json!({
            "input": sequence,
        });

        let response: OpenAiResponse = client
            .post(url.clone())
            .json(&input)
            .send()
            .await
            .map_err(InternalError::from_std)?
            .json()
            .await
            .map_err(InternalError::from_std)?;

        let embedding = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| InternalError::from_message("Invalid response format"))
            .map(|data| data.embedding)?;

        embedding.normalize().map_err(InternalError::from_std)
    }

    pub(crate) fn embedding_size(&self) -> usize {
        match &self.inner {
            InnerEmbedder::Pipeline(embedder) => embedder.embedding_size(),
            InnerEmbedder::Sagemaker { embedding_size, .. }
            | InnerEmbedder::OpenAi { embedding_size, .. } => *embedding_size,
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
        embedder.run(EmbeddingKind::Query, "test").await.unwrap();
    }
}
