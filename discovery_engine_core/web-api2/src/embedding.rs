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

use crate::utils::RelativePathBuf;

use serde::{Deserialize, Serialize};
use xayn_discovery_engine_ai::Embedding;
use xayn_discovery_engine_bert::{AveragePooler, AvgBert, Config as BertConfig};


use crate::{error::common::InternalError, server::SetupError};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[allow(dead_code)]
    #[serde(default = "default_directory")]
    directory: RelativePathBuf,
}

fn default_directory() -> RelativePathBuf {
    "assets".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            directory: default_directory(),
        }
    }
}

pub(crate) struct Embedder {
    #[allow(dead_code)]
    bert: AvgBert,
}

impl Embedder {
    pub(crate) fn run(&self, s: &str) -> Result<Embedding, InternalError> {
        self.smbert.run(s).map_err(InternalError::from_std)
    }

    pub(crate) fn load(config: &Config) -> Result<Self, SetupError> {
        let bert = BertConfig::new(&config.directory.relative())?
            .with_pooler::<AveragePooler>()
            .with_token_size(64)?
            .build()?;

        Ok(Embedder { bert })
    }
}
