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

use derive_more::{Deref, From};
use figment::value::Dict;
use ndarray::Array2;
use tokenizers::{
    tokenizer::Tokenizer as HfTokenizer,
    utils::{
        padding::{PaddingDirection, PaddingParams, PaddingStrategy},
        truncation::{TruncationDirection, TruncationParams, TruncationStrategy},
    },
    Error,
};
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

/// A pre-configured huggingface tokenizer.
pub struct Tokenizer {
    tokenizer: HfTokenizer,
    add_special_tokens: bool,
    use_type_ids: bool,
}

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let mut tokenizer = HfTokenizer::from_file(config.dir.join("tokenizer.json"))?;
        let padding_token = config.extract::<String>("tokenizer.tokens.padding")?;
        let padding = PaddingParams {
            strategy: PaddingStrategy::Fixed(config.token_size),
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: tokenizer
                .token_to_id(&padding_token)
                .ok_or("missing padding token")?,
            pad_type_id: 0,
            pad_token: padding_token,
        };
        let truncation = TruncationParams {
            direction: TruncationDirection::Right,
            max_length: config.token_size,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
        };
        tokenizer.with_padding(Some(padding));
        tokenizer.with_truncation(Some(truncation));
        let use_type_ids = config.extract::<Dict>("model.input")?.len() > 2;
        let add_special_tokens = config.extract::<bool>("tokenizer.add-special-tokens")?;

        Ok(Tokenizer {
            tokenizer,
            add_special_tokens,
            use_type_ids,
        })
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        let encoding = self
            .tokenizer
            .encode(sequence.as_ref(), self.add_special_tokens)?;
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        Ok(Encoding {
            token_ids: array_from(encoding.get_ids()),
            attention_mask: array_from(encoding.get_attention_mask()),
            type_ids: self
                .use_type_ids
                .then(|| array_from(encoding.get_type_ids())),
        })
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::{e5_mocked, smbert_mocked};

    use super::*;

    #[test]
    fn test_smbert() {
        let config = Config::new(smbert_mocked().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert!(encoding.token_ids.shape() == [1, 257]);
        let first_ids = encoding
            .token_ids
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>();
        assert!(
            first_ids
                == vec![
                    2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ]
        );
        let first_attention_mask = encoding
            .attention_mask
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>();
        assert!(
            first_attention_mask
                == vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        let first_type_ids = encoding
            .type_ids
            .unwrap()
            .iter()
            .take(20)
            .copied()
            .collect::<Vec<_>>();
        assert!(first_type_ids == vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_smbert_troublemakers() {
        let config = Config::new(smbert_mocked().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("for “life-threatening storm surge” according")
            .unwrap();
        assert!(encoding.token_ids.shape() == [1, 257]);
        let first_ids = encoding
            .token_ids
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<_>>();
        assert!(
            first_ids == vec![2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3, 0, 0, 0, 0]
        );
        let first_attention_mask = encoding
            .attention_mask
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<_>>();
        assert!(first_attention_mask == vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0]);
        let first_type_ids = encoding
            .type_ids
            .unwrap()
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<_>>();
        assert!(first_type_ids == vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_e5() {
        let config = Config::new(e5_mocked().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert!(encoding.token_ids.shape() == [1, 258]);
        let first_ids = encoding
            .token_ids
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<_>>();
        assert!(
            first_ids
                == vec![0, 32255, 621, 3638, 4, 39210, 19515, 20090, 24057, 142_766, 5, 2, 1, 1, 1]
        );
        let first_attention_mask = encoding
            .attention_mask
            .iter()
            .take(15)
            .copied()
            .collect::<Vec<_>>();
        assert!(first_attention_mask == vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0]);
        assert!(encoding.type_ids.is_none());
    }
}
