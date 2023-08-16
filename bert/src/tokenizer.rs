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
        let padding_token = config.extract::<String>("tokenizer.padding")?;
        let padding = PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
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
        let add_special_tokens = config.extract::<bool>("tokenizer.add_special_tokens")?;

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
        assert_eq!(encoding.get_ids().len(), 10);
        assert_eq!(
            encoding.get_ids(),
            [2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3],
        );
        assert!(encoding.get_attention_mask().iter().all(|v| *v == 1));
        assert!(encoding.get_type_ids().iter().all(|v| *v == 0));
    }

    #[test]
    fn test_smbert_truncation() {
        let token_size = 5;
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap())
            .unwrap()
            .with_token_size(token_size)
            .unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), token_size);
        assert_eq!(encoding.get_ids(), [2, 4538, 2128, 8561, 3]);
        assert!(encoding.get_attention_mask().iter().all(|v| *v == 1));
        assert!(encoding.get_type_ids().iter().all(|v| *v == 0));
    }

    #[test]
    fn test_smbert_troublemakers() {
        let config = Config::new(smbert_mocked().unwrap(), ort().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("for “life-threatening storm surge” according")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), 11);
        assert_eq!(
            encoding.get_ids(),
            [2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3],
        );
        assert!(encoding.get_attention_mask().iter().all(|v| *v == 1));
        assert!(encoding.get_type_ids().iter().all(|v| *v == 0));
    }

    #[test]
    fn test_e5() {
        let config = Config::new(e5_mocked().unwrap(), ort().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(encoding.get_ids().len(), 12);
        assert_eq!(
            encoding.get_ids(),
            [101, 2122, 2024, 3671, 1010, 2691, 7861, 8270, 4667, 2015, 1012, 102],
        );
        assert!(encoding.get_attention_mask().iter().all(|v| *v == 1));
        assert!(encoding.get_type_ids().iter().all(|v| *v == 0));
    }
}
