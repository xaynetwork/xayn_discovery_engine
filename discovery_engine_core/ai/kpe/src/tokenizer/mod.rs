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

pub(crate) mod encoding;
pub(crate) mod key_phrase;

use std::io::BufRead;

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

/// A pre-configured Bert tokenizer for key phrase extraction.
#[derive(Debug)]
pub(crate) struct Tokenizer<const KEY_PHRASE_SIZE: usize> {
    tokenizer: TokenizerImpl<
        WordPieceModel,
        BertNormalizer,
        BertPreTokenizer,
        BertProcessing,
        WordPieceDecoder,
    >,
    key_phrase_max_count: Option<usize>,
    key_phrase_min_score: Option<f32>,
}

impl<const KEY_PHRASE_SIZE: usize> Tokenizer<KEY_PHRASE_SIZE> {
    /// Creates a tokenizer from a vocabulary.
    ///
    /// Can be set to cleanse accents and to lowercase the sequences. Requires the maximum number of
    /// tokens per tokenized sequence, which applies to padding and truncation and includes special
    /// tokens as well.
    ///
    /// Optionally takes an upper count for the number of returned key phrases as well as a lower
    /// threshold for the scores of returned key phrases.
    pub(crate) fn new(
        vocab: impl BufRead,
        cleanse_accents: bool,
        lower_case: bool,
        token_size: usize,
        key_phrase_max_count: Option<usize>,
        key_phrase_min_score: Option<f32>,
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

        let tokenizer = TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(normalizer))
            .with_pre_tokenizer(Some(BertPreTokenizer))
            .with_post_processor(Some(post_processor))
            .with_padding(Some(padding))
            .with_truncation(Some(truncation))
            .with_decoder(Some(decoder))
            .build()?;

        Ok(Tokenizer {
            tokenizer,
            key_phrase_max_count,
            key_phrase_min_score,
        })
    }
}
