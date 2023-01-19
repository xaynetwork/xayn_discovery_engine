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
use xayn_ai_bert::{
    tokenizer::bert,
    AveragePooler,
    AvgBert,
    Config as BertConfig,
    NormalizedEmbedding,
};

use crate::{error::common::InternalError, server::SetupError, utils::RelativePathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_directory")]
    pub(crate) directory: RelativePathBuf,
    #[serde(default = "default_token_size")]
    pub(crate) token_size: usize,
}

fn default_directory() -> RelativePathBuf {
    "assets".into()
}

const fn default_token_size() -> usize {
    250
}

impl Default for Config {
    fn default() -> Self {
        Self {
            directory: default_directory(),
            token_size: default_token_size(),
        }
    }
}

pub(crate) struct Embedder {
    bert: AvgBert,
}

impl Embedder {
    pub(crate) fn run(&self, s: &str) -> Result<NormalizedEmbedding, InternalError> {
        self.bert
            .run(s)
            .map_err(InternalError::from_std)?
            .normalize()
            .map_err(InternalError::from_std)
    }

    pub(crate) fn load(config: &Config) -> Result<Self, SetupError> {
        let bert = BertConfig::new(config.directory.relative())?
            .with_tokenizer::<bert::Tokenizer>()
            .with_pooler::<AveragePooler>()
            .with_token_size(config.token_size)?
            .build()?;

        Ok(Embedder { bert })
    }
}
