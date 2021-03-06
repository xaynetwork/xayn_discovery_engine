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

use derive_more::{Deref, From};
use displaydoc::Display;
use ndarray::{Array1, Array2, Axis};
use thiserror::Error;
use xayn_discovery_engine_tokenizer::{
    AccentChars,
    Builder,
    BuilderError,
    CaseChars,
    ChineseChars,
    ControlChars,
    Padding,
    Tokenizer as BertTokenizer,
    Truncation,
};

/// A pre-configured Bert tokenizer.
#[derive(Debug)]
pub(crate) struct Tokenizer {
    tokenizer: BertTokenizer<i64>,
}

/// The potential errors of the tokenizer.
#[derive(Debug, Display, Error, PartialEq)]
pub enum TokenizerError {
    /// Failed to build the tokenizer: {0}
    Builder(#[from] BuilderError),
}

/// The token ids of the encoded sequence.
///
/// The token ids are of shape `(1, token_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct TokenIds(pub(crate) Array2<i64>);

/// The attention mask of the encoded sequence.
///
/// The attention mask is of shape `(1, token_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct AttentionMask(pub(crate) Array2<i64>);

/// The type ids of the encoded sequence.
///
/// The type ids are of shape `(1, token_size)`.
#[derive(Clone, Deref, From)]
pub(crate) struct TypeIds(pub(crate) Array2<i64>);

/// The encoded sequence.
pub(crate) struct Encoding {
    pub(crate) token_ids: TokenIds,
    pub(crate) attention_mask: AttentionMask,
    pub(crate) type_ids: TypeIds,
}

impl Tokenizer {
    /// Creates a tokenizer from a vocabulary.
    ///
    /// Can be set to keep accents and to lowercase the sequences. Requires the maximum number of
    /// tokens per tokenized sequence, which applies to padding and truncation and includes special
    /// tokens as well.
    pub(crate) fn new(
        // `BufRead` instead of `AsRef<Path>` is needed for wasm
        vocab: impl BufRead,
        accents: AccentChars,
        case: CaseChars,
        token_size: usize,
    ) -> Result<Self, TokenizerError> {
        let tokenizer = Builder::new(vocab)?
            .with_normalizer(ControlChars::Cleanse, ChineseChars::Keep, accents, case)
            .with_model("[UNK]", "##", 100)
            .with_post_tokenizer("[CLS]", "[SEP]")
            .with_truncation(Truncation::fixed(token_size, 0))
            .with_padding(Padding::fixed(token_size, "[PAD]"))
            .build()?;

        Ok(Tokenizer { tokenizer })
    }

    /// Encodes the sequence.
    ///
    /// The encoding is in correct shape for the model.
    pub(crate) fn encode(&self, sequence: impl AsRef<str>) -> Encoding {
        let encoding = self.tokenizer.encode(sequence);
        let (token_ids, type_ids, _, _, _, _, attention_mask, _) = encoding.into();

        let token_ids = Array1::from(token_ids).insert_axis(Axis(0)).into();
        let attention_mask = Array1::from(attention_mask).insert_axis(Axis(0)).into();
        let type_ids = Array1::from(type_ids).insert_axis(Axis(0)).into();

        Encoding {
            token_ids,
            attention_mask,
            type_ids,
        }
    }
}

#[cfg(test)]
mod tests {
    use ndarray::ArrayView;
    use std::{fs::File, io::BufReader};

    use xayn_discovery_engine_test_utils::smbert::vocab;
    use xayn_discovery_engine_tokenizer::{ModelError, PaddingError, PostTokenizerError};

    use super::*;

    #[test]
    fn test_vocab_empty() {
        assert_eq!(
            Tokenizer::new(
                Vec::new().as_slice(),
                AccentChars::Keep,
                CaseChars::Lower,
                10
            )
            .unwrap_err(),
            TokenizerError::Builder(BuilderError::Model(ModelError::EmptyVocab)),
        );
    }

    #[test]
    fn test_vocab_missing_cls() {
        let vocab = ["[SEP]", "[PAD]", "[UNK]", "a", "##b"].join("\n");
        assert_eq!(
            Tokenizer::new(vocab.as_bytes(), AccentChars::Keep, CaseChars::Lower, 10).unwrap_err(),
            TokenizerError::Builder(BuilderError::PostTokenizer(PostTokenizerError::ClsToken)),
        );
    }

    #[test]
    fn test_vocab_missing_sep() {
        let vocab = ["[CLS]", "[PAD]", "[UNK]", "a", "##b"].join("\n");
        assert_eq!(
            Tokenizer::new(vocab.as_bytes(), AccentChars::Keep, CaseChars::Lower, 10).unwrap_err(),
            TokenizerError::Builder(BuilderError::PostTokenizer(PostTokenizerError::SepToken)),
        );
    }

    #[test]
    fn test_vocab_missing_pad() {
        let vocab = ["[CLS]", "[SEP]", "[UNK]", "a", "##b"].join("\n");
        assert_eq!(
            Tokenizer::new(vocab.as_bytes(), AccentChars::Keep, CaseChars::Lower, 10).unwrap_err(),
            TokenizerError::Builder(BuilderError::Padding(PaddingError::PadToken)),
        );
    }

    #[test]
    fn test_vocab_missing_unk() {
        let vocab = ["[CLS]", "[SEP]", "[PAD]", "a", "##b"].join("\n");
        assert_eq!(
            Tokenizer::new(vocab.as_bytes(), AccentChars::Keep, CaseChars::Lower, 10).unwrap_err(),
            TokenizerError::Builder(BuilderError::Model(ModelError::UnkToken)),
        );
    }

    #[test]
    fn test_vocab_missing_prefix() {
        let vocab = ["[CLS]", "[SEP]", "[PAD]", "[UNK]", "a##b"].join("\n");
        assert_eq!(
            Tokenizer::new(vocab.as_bytes(), AccentChars::Keep, CaseChars::Lower, 10).unwrap_err(),
            TokenizerError::Builder(BuilderError::Model(ModelError::SubwordPrefix)),
        );
    }

    fn tokenizer(token_size: usize) -> Tokenizer {
        let vocab = BufReader::new(File::open(vocab().unwrap()).unwrap());
        let accents = AccentChars::Cleanse;
        let case = CaseChars::Lower;
        Tokenizer::new(vocab, accents, case, token_size).unwrap()
    }

    #[test]
    fn test_encode_short() {
        let shape = (1, 20);
        let encoding = tokenizer(shape.1).encode("These are normal, common EMBEDDINGS.");
        assert_eq!(
            encoding.token_ids.0,
            ArrayView::from_shape(
                shape,
                &[2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView::from_shape(
                shape,
                &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.type_ids.0,
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
        let encoding = tokenizer(shape.1).encode("These are normal, common EMBEDDINGS.");
        assert_eq!(
            encoding.token_ids.0,
            ArrayView::from_shape(shape, &[2, 4538, 2128, 8561, 1, 6541, 69469, 2762, 5, 3])
                .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]).unwrap(),
        );
        assert_eq!(
            encoding.type_ids.0,
            ArrayView::from_shape(shape, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        );
    }

    #[test]
    fn test_encode_troublemakers() {
        let shape = (1, 15);
        let encoding = tokenizer(shape.1).encode("for ???life-threatening storm surge??? according");
        assert_eq!(
            encoding.token_ids.0,
            ArrayView::from_shape(
                shape,
                &[2, 1665, 1, 3902, 1, 83775, 11123, 41373, 1, 7469, 3, 0, 0, 0, 0],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0]).unwrap(),
        );
        assert_eq!(
            encoding.type_ids.0,
            ArrayView::from_shape(shape, &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap(),
        );
    }
}
