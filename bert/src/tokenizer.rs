// Copyright 2021 Xayn AG
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

pub mod bert;

use derive_more::{Deref, From};
use ndarray::Array2;
use tokenizers::Error;
use tract_onnx::prelude::{tvec, TVec, Tensor};

use crate::config::Config;

/// The attention mask of the encoded sequence.
///
/// The attention mask is of shape `(1, token_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct AttentionMask(pub(crate) Array2<i64>);

/// The encoded sequence.
#[derive(Clone)]
pub struct Encoding {
    pub(crate) token_ids: Array2<i64>,
    pub(crate) attention_mask: Array2<i64>,
    pub(crate) type_ids: Array2<i64>,
}

impl Encoding {
    pub(crate) fn to_attention_mask(&self) -> AttentionMask {
        self.attention_mask.clone().into()
    }
}

impl From<Encoding> for TVec<Tensor> {
    fn from(encoding: Encoding) -> Self {
        tvec![
            encoding.token_ids.into(),
            encoding.attention_mask.into(),
            encoding.type_ids.into(),
        ]
    }
}

impl From<Encoding> for Vec<Array2<i64>> {
    fn from(encoding: Encoding) -> Self {
        vec![
            encoding.token_ids,
            encoding.attention_mask,
            encoding.type_ids,
        ]
    }
}

pub trait Tokenize {
    /// Creates a tokenizer from a configuration.
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error>
    where
        Self: Sized;

    /// Encodes the sequence.
    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error>;
}
