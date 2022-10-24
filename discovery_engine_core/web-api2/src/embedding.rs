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

#[derive(Debug, Deserialize, Serialize)]
pub struct Embedding {
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

impl Default for Embedding {
    fn default() -> Self {
        Self {
            vocabulary: default_vocabulary(),
            model: default_model(),
        }
    }
}
