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

use std::ops::ControlFlow;

use derive_more::{Deref, From};
use ndarray::Array2;
use tokenizers::{Encoding as BertEncoding, Error as TokenizerError, Offsets};

use crate::tokenizer::{key_phrase::KeyPhrases, Tokenizer};

/// The token ids of the encoded sequence.
///
/// The token ids are of shape `(1, token_size)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct TokenIds(pub(crate) Array2<i64>);

impl TokenIds {
    /// Checks if the token ids are valid, i.e. in the interval `[0, vocab_size)`.
    pub(crate) fn is_valid(&self, vocab_size: usize) -> bool {
        debug_assert!(i64::try_from(vocab_size as u64).is_ok());
        self.iter()
            .copied()
            .all(|token_id| 0 <= token_id && token_id < vocab_size as i64)
    }
}

/// The attention mask of the encoded sequence.
///
/// The attention mask is of shape `(1, token_size)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct AttentionMask(pub(crate) Array2<i64>);

impl AttentionMask {
    /// Checks if the attention mask is valid, i.e. either `0` or `1`.
    pub(crate) fn is_valid(&self) -> bool {
        self.iter()
            .copied()
            .all(|attention| attention == 0 || attention == 1)
    }
}

/// The starting tokens mask of the encoded sequence.
///
/// The valid mask is of shape `(token_size,)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct ValidMask(pub(crate) Vec<bool>);

impl ValidMask {
    /// Counts the number of valid entries in the mask.
    pub(crate) fn count(&self) -> usize {
        self.iter().filter(|valid| **valid).count()
    }

    /// Checks if the valid mask is valid, i.e. at least `key_phrase_size` valid entries.
    pub(crate) fn is_valid(&self, key_phrase_size: usize) -> bool {
        self.count() >= key_phrase_size
    }
}

/// The active words mask for each key phrase.
///
/// The active mask is of shape `(key_phrase_choices, key_phrase_mentions)`.
#[derive(Clone, Debug, Deref, From)]
pub(crate) struct ActiveMask(pub(crate) Array2<bool>);

impl ActiveMask {
    /// Checks if the active mask is valid, i.e. at least one mention per choice.
    pub(crate) fn is_valid(&self) -> bool {
        self.rows()
            .into_iter()
            .all(|mentions| mentions.iter().copied().any(|active| active))
    }
}

/// The encoded sequence.
#[derive(Clone, Debug)]
pub(crate) struct Encoding {
    pub(crate) token_ids: TokenIds,
    pub(crate) attention_mask: AttentionMask,
    pub(crate) valid_mask: ValidMask,
    pub(crate) active_mask: ActiveMask,
}

impl Encoding {
    /// Checks if all parts of the encoding are valid.
    pub(crate) fn is_valid(&self, vocab_size: usize, key_phrase_size: usize) -> bool {
        self.token_ids.is_valid(vocab_size)
            && self.attention_mask.is_valid()
            && self.valid_mask.is_valid(key_phrase_size)
            && self.active_mask.is_valid()
    }
}

impl<const KEY_PHRASE_SIZE: usize> Tokenizer<KEY_PHRASE_SIZE> {
    /// Encodes the sequence.
    ///
    /// The encoding is in correct shape for the models.
    pub(crate) fn encode(
        &self,
        sequence: impl AsRef<str>,
    ) -> Result<(Encoding, KeyPhrases<KEY_PHRASE_SIZE>), TokenizerError> {
        let sequence = sequence.as_ref();
        let encoding = self.tokenizer.encode(sequence, true)?;
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        let token_ids = array_from(encoding.get_ids()).into();
        let attention_mask = array_from(encoding.get_attention_mask()).into();

        let valid_mask = valid_mask(encoding.get_offsets());
        let words = decode_words(sequence, encoding.get_offsets(), encoding.get_overflowing());
        let key_phrases =
            KeyPhrases::collect(&words, self.key_phrase_max_count, self.key_phrase_min_score);
        let active_mask = key_phrases.active_mask();

        let encoding = Encoding {
            token_ids,
            attention_mask,
            valid_mask,
            active_mask,
        };

        encoding
            .is_valid(self.tokenizer.get_vocab_size(true), KEY_PHRASE_SIZE)
            .then_some((encoding, key_phrases))
            .ok_or_else(|| "invalid encoding".into())
    }
}

/// Decodes the tokenized words.
///
/// Joins starting tokens with their continuing tokens. Everything which is not separated by
/// whitespace is considered as continuation as well, e.g. punctuation. All words are lowercased.
fn decode_words(sequence: &str, offsets: &[Offsets], overflowing: &[BertEncoding]) -> Vec<String> {
    let mut words = Vec::<String>::with_capacity(offsets.len());
    let (word_start, word_end) = offsets.iter().fold(
        (0, 0),
        |(word_start, word_end), &(token_start, token_end)| {
            if word_end < token_start {
                words.push(sequence[word_start..word_end].to_lowercase());
                (token_start, token_end)
            } else {
                (
                    word_start,
                    // tokenizers uses (0, 0) instead of (n, n) as offsets for special tokens even
                    // if they aren't at the beginning of the sequence but at the n-th position
                    token_end.max(word_end),
                )
            }
        },
    );
    if word_start < word_end {
        if overflowing.is_empty() {
            words.push(sequence[word_start..word_end].to_lowercase());
        } else {
            // subtokens of the last word might have been truncated during tokenization, but we can
            // still use the whole word for the keyphrase because the model only pays attention to
            // the starting token
            if let ControlFlow::Continue((word_start, word_end)) = overflowing
                .as_ref()
                .iter()
                .flat_map(BertEncoding::get_offsets)
                .try_fold(
                    (word_start, word_end),
                    |(word_start, word_end), &(token_start, token_end)| {
                        if word_end < token_start {
                            words.push(sequence[word_start..word_end].to_lowercase());
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue((word_start, token_end.max(word_end)))
                        }
                    },
                )
            {
                words.push(sequence[word_start..word_end].to_lowercase());
            }
        }
    }
    words.shrink_to_fit();

    words
}

/// Creates the mask of starting tokens.
fn valid_mask(offsets: impl AsRef<[Offsets]>) -> ValidMask {
    offsets
        .as_ref()
        .iter()
        .scan(0, |previous_end, &(start, end)| {
            let valid = start > *previous_end || (start == 0 && end > 0);
            *previous_end = end;
            Some(valid)
        })
        .collect::<Vec<_>>()
        .into()
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::BufReader};

    use ndarray::ArrayView2;

    use super::*;
    use xayn_discovery_engine_test_utils::smbert::vocab;

    /// Tokens: This embedd ##ing fit ##s perfect ##ly .
    const EXACT_SEQUENCE: &str = "This embedding fits perfectly.";
    /// Tokens: This is an embedd ##ing .
    const SHORT_SEQUENCE: &str = "This is an embedding.";
    /// Tokens: This embedd ##ing is way too long .
    const LONG_SEQUENCE: &str = "This embedding is way too long.";

    fn tokenizer(token_size: usize) -> Tokenizer<3> {
        let vocab = BufReader::new(File::open(vocab().unwrap()).unwrap());
        let cleanse_accents = true;
        let lower_case = true;
        let key_phrase_count = None;
        let key_phrase_score = None;

        Tokenizer::new(
            vocab,
            cleanse_accents,
            lower_case,
            token_size,
            key_phrase_count,
            key_phrase_score,
        )
        .unwrap()
    }

    #[test]
    fn test_encode_exact() {
        let shape = (1, 10);
        let tokenizer = tokenizer(shape.1);
        let (encoding, _) = tokenizer.encode(EXACT_SEQUENCE).unwrap();
        assert_eq!(
            encoding.token_ids.0,
            ArrayView2::from_shape(
                shape,
                &[2, 2584, 69469, 1599, 13891, 1046, 18992, 1838, 5, 3],
            )
            .unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView2::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 1, 1]).unwrap(),
        );
        assert_eq!(
            encoding.valid_mask.0,
            [false, true, true, false, true, false, true, false, false, false],
        );
        assert_eq!(
            encoding.active_mask.0,
            ArrayView2::from_shape(
                (9, 9),
                &[
                    true, false, false, false, false, false, false, false, false, //
                    false, true, false, false, false, false, false, false, false, //
                    false, false, true, false, false, false, false, false, false, //
                    false, false, false, true, false, false, false, false, false, //
                    false, false, false, false, true, false, false, false, false, //
                    false, false, false, false, false, true, false, false, false, //
                    false, false, false, false, false, false, true, false, false, //
                    false, false, false, false, false, false, false, true, false, //
                    false, false, false, false, false, false, false, false, true, //
                ],
            )
            .unwrap(),
        );
    }

    #[test]
    fn test_encode_padded() {
        let shape = (1, 10);
        let tokenizer = tokenizer(shape.1);
        let (encoding, _) = tokenizer.encode(SHORT_SEQUENCE).unwrap();
        assert_eq!(
            encoding.token_ids.0,
            ArrayView2::from_shape(shape, &[2, 2584, 1693, 1624, 69469, 1599, 5, 3, 0, 0]).unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView2::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1, 0, 0]).unwrap(),
        );
        assert_eq!(
            encoding.valid_mask.0,
            [false, true, true, true, true, false, false, false, false, false],
        );
        assert_eq!(
            encoding.active_mask.0,
            ArrayView2::from_shape(
                (9, 9),
                &[
                    true, false, false, false, false, false, false, false, false, //
                    false, true, false, false, false, false, false, false, false, //
                    false, false, true, false, false, false, false, false, false, //
                    false, false, false, true, false, false, false, false, false, //
                    false, false, false, false, true, false, false, false, false, //
                    false, false, false, false, false, true, false, false, false, //
                    false, false, false, false, false, false, true, false, false, //
                    false, false, false, false, false, false, false, true, false, //
                    false, false, false, false, false, false, false, false, true, //
                ],
            )
            .unwrap(),
        );
    }

    #[test]
    fn test_encode_truncated() {
        let shape = (1, 8);
        let tokenizer = tokenizer(shape.1);
        let (encoding, _) = tokenizer.encode(LONG_SEQUENCE).unwrap();
        assert_eq!(
            encoding.token_ids.0,
            ArrayView2::from_shape(shape, &[2, 2584, 69469, 1599, 1693, 5331, 11700, 3]).unwrap(),
        );
        assert_eq!(
            encoding.attention_mask.0,
            ArrayView2::from_shape(shape, &[1, 1, 1, 1, 1, 1, 1, 1]).unwrap(),
        );
        assert_eq!(
            encoding.valid_mask.0,
            [false, true, true, false, true, true, true, false],
        );
        assert_eq!(
            encoding.active_mask.0,
            ArrayView2::from_shape(
                (12, 12),
                &[
                    true, false, false, false, false, false, false, false, false, false, false,
                    false, //
                    false, true, false, false, false, false, false, false, false, false, false,
                    false, //
                    false, false, true, false, false, false, false, false, false, false, false,
                    false, //
                    false, false, false, true, false, false, false, false, false, false, false,
                    false, //
                    false, false, false, false, true, false, false, false, false, false, false,
                    false, //
                    false, false, false, false, false, true, false, false, false, false, false,
                    false, //
                    false, false, false, false, false, false, true, false, false, false, false,
                    false, //
                    false, false, false, false, false, false, false, true, false, false, false,
                    false, //
                    false, false, false, false, false, false, false, false, true, false, false,
                    false, //
                    false, false, false, false, false, false, false, false, false, true, false,
                    false, //
                    false, false, false, false, false, false, false, false, false, false, true,
                    false, //
                    false, false, false, false, false, false, false, false, false, false, false,
                    true, //
                ],
            )
            .unwrap(),
        );
    }

    const EXACT_WORDS: [&str; 4] = ["this", "embedding", "fits", "perfectly."];
    const SHORT_WORDS: [&str; 4] = ["this", "is", "an", "embedding."];
    const LONG_WORDS: [&str; 6] = ["this", "embedding", "is", "way", "too", "long."];

    #[test]
    fn test_decode_words_exact() {
        let encoding = tokenizer(10)
            .tokenizer
            .encode(EXACT_SEQUENCE, true)
            .unwrap();
        let words = decode_words(
            EXACT_SEQUENCE,
            encoding.get_offsets(),
            encoding.get_overflowing(),
        );
        assert_eq!(words, EXACT_WORDS);
    }

    #[test]
    fn test_decode_words_padded() {
        let encoding = tokenizer(10)
            .tokenizer
            .encode(SHORT_SEQUENCE, true)
            .unwrap();
        let words = decode_words(
            SHORT_SEQUENCE,
            encoding.get_offsets(),
            encoding.get_overflowing(),
        );
        assert_eq!(words, SHORT_WORDS);
    }

    #[test]
    fn test_decode_words_truncated_between() {
        let encoding = tokenizer(8).tokenizer.encode(LONG_SEQUENCE, true).unwrap();
        let words = decode_words(
            LONG_SEQUENCE,
            encoding.get_offsets(),
            encoding.get_overflowing(),
        );
        assert_eq!(words, LONG_WORDS[..5]);
    }

    #[test]
    fn test_decode_words_truncated_within() {
        let encoding = tokenizer(4).tokenizer.encode(LONG_SEQUENCE, true).unwrap();
        let words = decode_words(
            LONG_SEQUENCE,
            encoding.get_offsets(),
            encoding.get_overflowing(),
        );
        assert_eq!(words, LONG_WORDS[..2]);
    }

    #[test]
    fn test_decode_words_truncated_empty() {
        let encoding = tokenizer(2).tokenizer.encode(LONG_SEQUENCE, true).unwrap();
        let words = decode_words(
            LONG_SEQUENCE,
            encoding.get_offsets(),
            encoding.get_overflowing(),
        );
        assert!(words.is_empty());
    }

    #[test]
    fn test_decode_words_empty() {
        let encoding = tokenizer(5).tokenizer.encode("", true).unwrap();
        let words = decode_words("", encoding.get_offsets(), encoding.get_overflowing());
        assert!(words.is_empty());
    }

    #[test]
    fn test_valid_mask_full() {
        let encoding = tokenizer(10)
            .tokenizer
            .encode(EXACT_SEQUENCE, true)
            .unwrap();
        assert_eq!(
            valid_mask(encoding.get_offsets()).0,
            [false, true, true, false, true, false, true, false, false, false],
        );
    }

    #[test]
    fn test_valid_mask_empty() {
        assert!(valid_mask(&[]).is_empty());
    }
}
