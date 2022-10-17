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

use std::io::BufRead;
#[cfg(feature = "japanese")]
use std::path::PathBuf;

use derive_more::{Deref, From};
#[cfg(feature = "japanese")]
use itertools::Itertools;
#[cfg(feature = "japanese")]
use lindera::{
    mode::Mode as JapaneseMode,
    tokenizer::{
        DictionaryConfig as JapaneseDictionaryConfig,
        DictionaryKind as JapaneseDictionaryKind,
        DictionarySourceType as JapaneseDictionarySourceType,
        Tokenizer as JapanesePreTokenizer,
        TokenizerConfig as JapanesePreTokenizerConfig,
        UserDictionaryConfig as JapaneseUserDictionaryConfig,
    },
};
use ndarray::Array2;
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
    Error as TokenizerError,
    Model,
    TokenizerBuilder,
    TokenizerImpl,
};
use tract_onnx::prelude::{tvec, TVec, Tensor};

/// A pre-configured Bert tokenizer.
pub struct Tokenizer {
    bert: TokenizerImpl<
        WordPieceModel,
        BertNormalizer,
        BertPreTokenizer,
        BertProcessing,
        WordPieceDecoder,
    >,
    #[cfg(feature = "japanese")]
    japanese: JapanesePreTokenizer,
}

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

impl Tokenizer {
    /// Creates a tokenizer from a vocabulary.
    ///
    /// Can be set to cleanse accents and to lowercase the sequences. Requires the maximum number of
    /// tokens per tokenized sequence, which applies to padding and truncation and includes special
    /// tokens as well.
    pub fn new(
        vocab: impl BufRead,
        #[cfg(feature = "japanese")] japanese: Option<PathBuf>,
        cleanse_accents: bool,
        lower_case: bool,
        token_size: usize,
    ) -> Result<Self, TokenizerError> {
        let vocab = vocab
            .lines()
            .enumerate()
            .map(|(idx, word)| Ok((word?.trim().to_string(), u32::try_from(idx)?)))
            .collect::<Result<_, TokenizerError>>()?;
        let model = WordPieceBuilder::new()
            .vocab(vocab)
            .unk_token("[UNK]".into())
            .continuing_subword_prefix("##".into())
            .max_input_chars_per_word(100)
            .build()?;
        let normalizer = BertNormalizer::new(true, false, Some(cleanse_accents), lower_case);
        let post_processor = BertProcessing::new(
            (
                "[SEP]".into(),
                model.token_to_id("[SEP]").ok_or("missing sep token")?,
            ),
            (
                "[CLS]".into(),
                model.token_to_id("[CLS]").ok_or("missing cls token")?,
            ),
        );
        let padding = PaddingParams {
            strategy: PaddingStrategy::Fixed(token_size),
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: 0,
            pad_type_id: 0,
            pad_token: "[PAD]".into(),
        };
        let truncation = TruncationParams {
            direction: TruncationDirection::Right,
            max_length: token_size,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
        };
        let decoder = WordPieceDecoder::new("##".into(), true);

        let bert = TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(normalizer))
            .with_pre_tokenizer(Some(BertPreTokenizer))
            .with_post_processor(Some(post_processor))
            .with_padding(Some(padding))
            .with_truncation(Some(truncation))
            .with_decoder(Some(decoder))
            .build()?;

        #[cfg(feature = "japanese")]
        let japanese = JapanesePreTokenizer::with_config(JapanesePreTokenizerConfig {
            dictionary: JapaneseDictionaryConfig {
                kind: JapaneseDictionaryKind::IPADIC,
                path: None,
            },
            user_dictionary: japanese.map(|path| JapaneseUserDictionaryConfig {
                kind: JapaneseDictionaryKind::IPADIC,
                source_type: JapaneseDictionarySourceType::Csv,
                path,
            }),
            mode: JapaneseMode::Normal,
        })?;

        Ok(Tokenizer {
            bert,
            #[cfg(feature = "japanese")]
            japanese,
        })
    }

    /// Encodes the sequence.
    ///
    /// The encoding is in correct shape for the model.
    pub fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, TokenizerError> {
        let sequence = sequence.as_ref();
        #[cfg(feature = "japanese")]
        #[allow(unstable_name_collisions)]
        let sequence = self
            .japanese
            .tokenize(sequence)?
            .into_iter()
            .map(|token| token.text)
            .intersperse(" ")
            .collect::<String>();

        let encoding = self.bert.encode(sequence, true)?;
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        Ok(Encoding {
            token_ids: array_from(encoding.get_ids()),
            attention_mask: array_from(encoding.get_attention_mask()),
            type_ids: array_from(encoding.get_type_ids()),
        })
    }
}

#[cfg(test)]
mod tests {
    use ndarray::ArrayView;
    use std::{fs::File, io::BufReader};

    use xayn_discovery_engine_test_utils::smbert::vocab;

    use super::*;

    fn tokenizer(token_size: usize) -> Tokenizer {
        let vocab = BufReader::new(File::open(vocab().unwrap()).unwrap());
        let cleanse_accents = true;
        let lower_case = true;
        Tokenizer::new(
            vocab,
            #[cfg(feature = "japanese")]
            None,
            cleanse_accents,
            lower_case,
            token_size,
        )
        .unwrap()
    }

    #[test]
    fn test_encode_short() {
        let shape = (1, 20);
        let encoding = tokenizer(shape.1)
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(
            encoding.token_ids,
            ArrayView::from_shape(
                shape,
                &[2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask,
            ArrayView::from_shape(
                shape,
                &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.type_ids,
            ArrayView::from_shape(
                shape,
                &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
        );
    }

    #[test]
    fn test_encode_long() {
        let shape = (1, 10);
        let encoding = tokenizer(shape.1)
            .encode("These are normal, common EMBEDDINGS.")
            .unwrap();
        assert_eq!(
            encoding.token_ids,
            ArrayView::from_shape(shape, &[2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3])
                .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask,
            ArrayView::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]).unwrap(),
        );
        assert_eq!(
            encoding.type_ids,
            ArrayView::from_shape(shape, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        );
    }

    #[test]
    fn test_encode_troublemakers() {
        let shape = (1, 15);
        let encoding = tokenizer(shape.1)
            .encode("for “life-threatening storm surge” according")
            .unwrap();
        assert_eq!(
            encoding.token_ids,
            ArrayView::from_shape(
                shape,
                &[2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask,
            ArrayView::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0]).unwrap(),
        );
        assert_eq!(
            encoding.type_ids,
            ArrayView::from_shape(shape, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        );
    }
}
