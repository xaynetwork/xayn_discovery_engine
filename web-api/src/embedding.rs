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

use serde::{Deserialize, Deserializer, Serialize};
use xayn_ai_bert::{AvgEmbedder, Config as EmbedderConfig, NormalizedEmbedding};

use crate::{app::SetupError, error::common::InternalError, utils::RelativePathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Config {
    Pipeline(Pipeline),
}

impl Default for Config {
    fn default() -> Self {
        Self::Pipeline(Pipeline::default())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Pipeline {
    #[serde(deserialize_with = "deserialize_relative_path_buf")]
    pub(crate) directory: RelativePathBuf,
    pub(crate) token_size: usize,
}

fn deserialize_relative_path_buf<'de, D>(deserializer: D) -> Result<RelativePathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    Ok(RelativePathBuf::from(buf))
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            directory: "assets".into(),
            token_size: 250,
        }
    }
}

pub(crate) enum Embedder {
    Pipeline(AvgEmbedder),
}

impl Embedder {
    #[allow(clippy::unused_async)]
    pub(crate) async fn load(config: &Config) -> Result<Self, SetupError> {
        match config {
            Config::Pipeline(Pipeline {
                directory,
                token_size,
            }) => Self::load_pipeline(directory, *token_size),
        }
    }

    fn load_pipeline(directory: &RelativePathBuf, token_size: usize) -> Result<Self, SetupError> {
        let config = EmbedderConfig::new(directory.relative())?
            .with_token_size(token_size)?
            .with_pooler();
        config.validate()?;
        let embedder = config.build()?;

        Ok(Self::Pipeline(embedder))
    }

    #[allow(clippy::unused_async)]
    pub(crate) async fn run(&self, sequence: &str) -> Result<NormalizedEmbedding, InternalError> {
        match self {
            Embedder::Pipeline(embedder) => embedder
                .run(sequence)
                .map_err(InternalError::from_std)?
                .normalize()
                .map_err(InternalError::from_std),
        }
    }

    pub(crate) fn embedding_size(&self) -> usize {
        match self {
            Embedder::Pipeline(embedder) => embedder.embedding_size(),
        }
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::xaynia;

    use super::*;

    #[tokio::test]
    async fn test_embedder() {
        let config = Config::Pipeline(Pipeline {
            directory: xaynia().unwrap().into(),
            ..Pipeline::default()
        });
        let embedder = Embedder::load(&config).await.unwrap();
        embedder.run("test").await.unwrap();
    }
}
