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

use anyhow::anyhow;
use tokenizers::{
    tokenizer::Tokenizer as HfTokenizer,
    utils::{
        padding::{PaddingDirection, PaddingParams, PaddingStrategy},
        truncation::{TruncationDirection, TruncationParams, TruncationStrategy},
    },
    Encoding,
    Error,
};

use crate::config::Config;

/// A pre-configured huggingface tokenizer.
pub(crate) struct Tokenizer {
    tokenizer: HfTokenizer,
    add_special_tokens: bool,
}

impl Tokenizer {
    pub(crate) fn new<P>(config: &Config<P>) -> Result<Self, Error> {
        let tokenizer = config.dir.join("tokenizer.json");
        if !tokenizer.exists() {
            return Err(
                anyhow!("embedder tokenizer '{}' doesn't exist", tokenizer.display()).into(),
            );
        }
        let mut tokenizer = HfTokenizer::from_file(tokenizer)?;
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
        let add_special_tokens = config.extract::<bool>("tokenizer.add-special-tokens")?;

        Ok(Tokenizer {
            tokenizer,
            add_special_tokens,
        })
    }

    pub(crate) fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        self.tokenizer
            .encode(sequence.as_ref(), self.add_special_tokens)
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::{e5_mocked, ort, smbert_mocked};

    use super::*;

    #[test]
    fn test_smbert() {
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), 257);
        assert_eq!(
            encoding.get_ids()[..20],
            [2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_attention_mask()[..20],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_type_ids()[..20],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
    }

    #[test]
    fn test_smbert_troublemakers() {
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("for “life-threatening storm surge” according")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), 257);
        assert_eq!(
            encoding.get_ids()[..15],
            [2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_attention_mask()[..15],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_type_ids()[..15],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
    }

    #[test]
    fn test_e5() {
        let config = Config::new(e5_mocked().unwrap(), ort().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), 258);
        assert_eq!(
            encoding.get_ids()[..15],
            [0, 32255, 621, 3638, 4, 39210, 19515, 20090, 24057, 142_766, 5, 2, 1, 1, 1],
        );
        assert_eq!(
            encoding.get_attention_mask()[..15],
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_type_ids()[..15],
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
    }
}
