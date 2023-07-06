// Copyright 2023 Xayn AG
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

use crate::{
    config::Config,
    tokenizer::{Encoding, Tokenize},
};

/// A pre-configured huggingface tokenizer.
pub struct Tokenizer {
    hf_tokenizer: HfTokenizer,
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
            hf_tokenizer: tokenizer,
            add_special_tokens,
            use_type_ids,
        })
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        let tokens = self
            .hf_tokenizer
            .encode(sequence.as_ref(), self.add_special_tokens)?;
        let token_ids: Vec<u32> = tokens.get_ids().to_vec();
        let attention_mask = tokens.get_attention_mask().to_vec();
        let type_ids = tokens.get_type_ids().to_vec();
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        Ok(Encoding {
            token_ids: array_from(&token_ids),
            attention_mask: array_from(&attention_mask),
            type_ids: if self.use_type_ids {
                Some(array_from(&type_ids))
            } else {
                None
            },
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
