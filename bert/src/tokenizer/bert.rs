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

use std::{
    borrow::Cow,
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
};

use itertools::Itertools;
use lindera::{
    dictionary::DictionaryConfig as JapaneseDictionaryConfig,
    mode::Mode as JapaneseMode,
    tokenizer::{Tokenizer as JapanesePreTokenizer, TokenizerConfig as JapanesePreTokenizerConfig},
};
use tokenizers::{
    decoders::wordpiece::WordPiece as WordPieceDecoder,
    models::wordpiece::{WordPiece as WordPieceModel, WordPieceBuilder},
    normalizers::bert::BertNormalizer,
    pre_tokenizers::bert::BertPreTokenizer,
    processors::bert::BertProcessing,
    utils::{
        padding::{PaddingDirection, PaddingParams, PaddingStrategy},
        truncation::{TruncationDirection, TruncationParams, TruncationStrategy},
    },
    Error,
    Model,
    TokenizerBuilder,
    TokenizerImpl,
};
use tract_onnx::prelude::{tvec, TValue, TVec};

use crate::{
    config::Config,
    tokenizer::{tvalue_from, Encoding, Tokenize},
};

/// A pre-configured Bert tokenizer.
pub struct Tokenizer {
    japanese: Option<JapanesePreTokenizer>,
    bert: TokenizerImpl<
        WordPieceModel,
        BertNormalizer,
        BertPreTokenizer,
        BertProcessing,
        WordPieceDecoder,
    >,
    special_token_ids: [u32; 4],
}

impl Tokenizer {
    fn to_input(encoding: &tokenizers::Encoding) -> TVec<TValue> {
        tvec![
            tvalue_from(encoding.get_ids()),
            tvalue_from(encoding.get_attention_mask()),
            tvalue_from(encoding.get_type_ids()),
        ]
    }
}

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let japanese = config
            .extract::<String>("pre-tokenizer.path")
            .ok()
            .map(|mecab| {
                JapanesePreTokenizer::from_config(JapanesePreTokenizerConfig {
                    dictionary: JapaneseDictionaryConfig {
                        kind: None,
                        path: Some(config.dir.join(mecab)),
                    },
                    user_dictionary: None,
                    mode: JapaneseMode::Normal,
                })
            })
            .transpose()?;

        let vocab = BufReader::new(File::open(config.dir.join("vocab.txt"))?)
            .lines()
            .enumerate()
            .map(|(idx, word)| Ok((word?.trim().to_string(), u32::try_from(idx)?)))
            .collect::<Result<HashMap<_, _>, Error>>()?;
        let unknown_token = config.extract::<String>("tokenizer.tokens.unknown")?;
        let unknown_id = vocab
            .get(&unknown_token)
            .copied()
            .ok_or("missing unknown token")?;
        let model = WordPieceBuilder::new()
            .vocab(vocab)
            .unk_token(unknown_token)
            .continuing_subword_prefix(config.extract("tokenizer.tokens.continuation")?)
            .max_input_chars_per_word(config.extract("tokenizer.max-chars")?)
            .build()?;
        let normalizer = BertNormalizer::new(
            config.extract("tokenizer.cleanse-text")?,
            false,
            Some(config.extract("tokenizer.cleanse-accents")?),
            config.extract("tokenizer.lower-case")?,
        );
        let separation_token = config.extract::<String>("tokenizer.tokens.separation")?;
        let separation_id = model
            .token_to_id(&separation_token)
            .ok_or("missing separation token")?;
        let class_token = config.extract::<String>("tokenizer.tokens.class")?;
        let class_id = model
            .token_to_id(&class_token)
            .ok_or("missing class token")?;
        let post_processor =
            BertProcessing::new((separation_token, separation_id), (class_token, class_id));
        let padding_token = config.extract::<String>("tokenizer.tokens.padding")?;
        let padding_id = model
            .token_to_id(&padding_token)
            .ok_or("missing padding token")?;
        let padding = PaddingParams {
            strategy: PaddingStrategy::Fixed(config.token_size),
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: padding_id,
            pad_type_id: 0,
            pad_token: padding_token,
        };
        let truncation = TruncationParams {
            direction: TruncationDirection::Right,
            max_length: config.token_size,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
        };

        let bert = TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(normalizer))
            .with_pre_tokenizer(Some(BertPreTokenizer))
            .with_post_processor(Some(post_processor))
            .with_padding(Some(padding))
            .with_truncation(Some(truncation))
            .build()?;
        let special_token_ids = [unknown_id, separation_id, class_id, padding_id];

        Ok(Tokenizer {
            japanese,
            bert,
            special_token_ids,
        })
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        #[allow(unstable_name_collisions)]
        let sequence = if let Some(japanese) = &self.japanese {
            japanese
                .tokenize(sequence.as_ref())?
                .into_iter()
                .map(|token| token.text)
                .intersperse(" ")
                .collect::<String>()
                .into()
        } else {
            Cow::Borrowed(sequence.as_ref())
        };

        Ok(Encoding {
            encoding: self.bert.encode(sequence, true)?,
            to_input: Self::to_input,
        })
    }

    fn special_token_ids(&self) -> &[u32] {
        &self.special_token_ids
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::{sjbert, smbert_mocked};

    use super::*;

    fn tokenizer(token_size: usize) -> Tokenizer {
        let config = Config::new(smbert_mocked().unwrap())
            .unwrap()
            .with_token_size(token_size)
            .unwrap();
        Tokenizer::new(&config).unwrap()
    }

    #[test]
    fn test_new_multi() {
        let multi = tokenizer(42);
        assert!(multi.japanese.is_none());
        assert!(multi.bert.get_normalizer().is_some());
        assert!(multi.bert.get_pre_tokenizer().is_some());
        assert!(multi.bert.get_post_processor().is_some());
        assert!(multi.bert.get_padding().is_some());
        assert!(multi.bert.get_truncation().is_some());
        assert!(multi.bert.get_decoder().is_none());
    }

    #[test]
    fn test_new_japan() {
        let config = Config::new(sjbert().unwrap())
            .unwrap()
            .with_token_size(42)
            .unwrap();
        let japan = Tokenizer::new(&config).unwrap();
        assert!(japan.japanese.is_some());
        assert!(japan.bert.get_normalizer().is_some());
        assert!(japan.bert.get_pre_tokenizer().is_some());
        assert!(japan.bert.get_post_processor().is_some());
        assert!(japan.bert.get_padding().is_some());
        assert!(japan.bert.get_truncation().is_some());
        assert!(japan.bert.get_decoder().is_none());
    }

    #[test]
    fn test_encode_short() {
        let shape = (1, 20);
        let encoding = tokenizer(shape.1)
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(
            encoding.get_ids(),
            [2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_attention_mask(),
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_type_ids(),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
    }

    #[test]
    fn test_encode_long() {
        let shape = (1, 10);
        let encoding = tokenizer(shape.1)
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(
            encoding.get_ids(),
            [2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3],
        );
        assert_eq!(
            encoding.get_attention_mask(),
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        );
        assert_eq!(encoding.get_type_ids(), [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_encode_troublemakers() {
        let shape = (1, 15);
        let encoding = tokenizer(shape.1)
            .encode("for “life-threatening storm surge” according")
            .unwrap();
        assert_eq!(
            encoding.get_ids(),
            [2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_attention_mask(),
            [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0],
        );
        assert_eq!(
            encoding.get_type_ids(),
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
    }
}
