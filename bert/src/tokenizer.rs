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

pub mod huggingface;

use derive_more::{Deref, From};
use ndarray::Array2;
use tokenizers::Error;
use tract_onnx::prelude::{tvec, IntoArcTensor, TValue, TVec};

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
    pub(crate) type_ids: Option<Array2<i64>>,
}

impl Encoding {
    pub(crate) fn to_attention_mask(&self) -> AttentionMask {
        self.attention_mask.clone().into()
    }
}

impl From<Encoding> for TVec<TValue> {
    fn from(encoding: Encoding) -> Self {
        let token_ids = TValue::Const(encoding.token_ids.into_arc_tensor());
        let attention_mask = TValue::Const(encoding.attention_mask.into_arc_tensor());
        if let Some(type_ids) = encoding
            .type_ids
            .map(|type_ids| TValue::Const(type_ids.into_arc_tensor()))
        {
            tvec![token_ids, attention_mask, type_ids]
        } else {
            tvec![token_ids, attention_mask]
        }
    }
}

impl From<Encoding> for Vec<Array2<i64>> {
    fn from(encoding: Encoding) -> Self {
        if let Some(type_ids) = encoding.type_ids {
            vec![encoding.token_ids, encoding.attention_mask, type_ids]
        } else {
            vec![encoding.token_ids, encoding.attention_mask]
        }
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
