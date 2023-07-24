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

use serde::{Deserialize, Serialize};
use xayn_ai_bert::{AvgEmbedder, Config as EmbedderConfig, NormalizedEmbedding};

use crate::{app::SetupError, error::common::InternalError, utils::RelativePathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub(crate) directory: RelativePathBuf,
    pub(crate) token_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            directory: "assets".into(),
            token_size: 250,
        }
    }
}

pub(crate) struct Embedder {
    embedder: AvgEmbedder,
}

impl Embedder {
    pub(crate) fn load(config: &Config) -> Result<Self, SetupError> {
        let config = EmbedderConfig::new(config.directory.relative())?
            .with_token_size(config.token_size)?
            .with_pooler();
        config.validate()?;
        let embedder = config.build()?;

        Ok(Self { embedder })
    }

    pub(crate) fn run(&self, sequence: &str) -> Result<NormalizedEmbedding, InternalError> {
        self.embedder
            .run(sequence)
            .map_err(InternalError::from_std)?
            .normalize()
            .map_err(InternalError::from_std)
    }

    pub(crate) fn embedding_size(&self) -> usize {
        self.embedder.embedding_size()
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::xaynia;

    use super::*;

    #[test]
    fn test_embedder() {
        let config = Config {
            directory: xaynia().unwrap().into(),
            ..Config::default()
        };
        let embedder = Embedder::load(&config).unwrap();
        embedder.run("test").unwrap();
    }
}
