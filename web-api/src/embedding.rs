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

use anyhow::bail;
use serde::{Deserialize, Serialize};
use xayn_ai_bert::{
    tokenizer::bert,
    AveragePooler,
    AvgBert,
    Config as BertConfig,
    NormalizedEmbedding,
    NormalizedSparseEmbedding,
};
use xayn_web_api_shared::SetupError;

use crate::{error::common::InternalError, utils::RelativePathBuf};

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
    bert: AvgBert,
}

impl Embedder {
    pub(crate) fn load(config: &Config) -> Result<Self, SetupError> {
        let path = config.directory.relative();
        if !path.exists() {
            bail!(
                "Fail to load Embedder: asset dir missing: {}",
                path.display()
            );
        }
        let config_file = path.join("config.toml");
        if !config_file.exists() {
            bail!(
                "Fail to load Embedder: <assets>/config.toml doesn't exist: {}",
                config_file.display()
            );
        }
        let bert = BertConfig::new(path)?
            .with_tokenizer::<bert::Tokenizer>()
            .with_pooler::<AveragePooler>()
            .with_token_size(config.token_size)?
            .build()?;

        Ok(Embedder { bert })
    }

    pub(crate) fn embedding_size(&self) -> usize {
        self.bert.embedding_size()
    }

    pub(crate) fn run(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<NormalizedEmbedding, InternalError> {
        self.bert
            .run(sequence)
            .map_err(InternalError::from_std)?
            .normalize()
            .map_err(InternalError::from_std)
    }

    pub(crate) fn run_sparse_document(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<NormalizedSparseEmbedding, InternalError> {
        self.bert
            .run_sparse_document(sequence)
            .map_err(InternalError::from_std)?
            .normalize()
            .map_err(InternalError::from_std)
    }

    pub(crate) fn run_sparse_query(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<NormalizedSparseEmbedding, InternalError> {
        self.bert
            .run_sparse_query(sequence)
            .map_err(InternalError::from_std)?
            .normalize()
            .map_err(InternalError::from_std)
    }
}
