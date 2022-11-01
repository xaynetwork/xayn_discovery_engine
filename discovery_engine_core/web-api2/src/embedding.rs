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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xayn_discovery_engine_bert::{AveragePooler, SMBert, SMBertConfig};

use crate::server::SetupError;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[allow(dead_code)]
    #[serde(default = "default_vocabulary")]
    vocabulary: PathBuf,
    #[allow(dead_code)]
    #[serde(default = "default_model")]
    model: PathBuf,
}

fn default_vocabulary() -> PathBuf {
    "assets/vocab.txt".into()
}

fn default_model() -> PathBuf {
    "assets/model.onnx".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vocabulary: default_vocabulary(),
            model: default_model(),
        }
    }
}

pub(crate) struct Embedder {
    #[allow(dead_code)]
    smbert: SMBert,
}

impl Embedder {
    pub(crate) fn load(config: &Config) -> Result<Self, SetupError> {
        let smbert = SMBertConfig::from_files(&config.vocabulary, &config.model)?
            .with_cleanse_accents(true)
            .with_lower_case(true)
            .with_pooling::<AveragePooler>()
            .with_token_size(64)?
            .build()?;

        Ok(Embedder { smbert })
    }
}
