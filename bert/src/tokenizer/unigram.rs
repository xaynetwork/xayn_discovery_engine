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
    fs::{read_to_string, File},
    io::BufReader,
};

use ndarray::Array2;
use tokenizers::{
    models::unigram::Unigram,
    normalizers::Precompiled,
    pre_tokenizers::{
        metaspace::Metaspace,
        sequence::Sequence,
        whitespace::WhitespaceSplit,
        PreTokenizerWrapper,
    },
    processors::template::{TemplateProcessing, TemplateProcessingBuilder},
    utils::{
        padding::{PaddingDirection, PaddingParams, PaddingStrategy},
        truncation::{TruncationDirection, TruncationParams, TruncationStrategy},
    },
    Error,
    Model,
    TokenizerBuilder,
    TokenizerImpl,
};

use crate::{
    config::Config,
    tokenizer::{Encoding, Tokenize},
};

/// A pre-configured E5 tokenizer.
pub struct Tokenizer{
    hf_tokenizer: Unigram,
}

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let model = Unigram::load(config.dir.join("tokenizer.json"))?;
        Ok(Tokenizer{hf_tokenizer: model})
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        let tokens = self.hf_tokenizer.encode(sequence.as_ref())?;
        let token_ids: Vec<u32> = tokens.iter().filter_map(|token| self.hf_tokenizer.token_to_id(token)).collect();
        let attention_mask = vec![1; tokens.len()];
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        Ok(Encoding {
            token_ids: array_from(&token_ids),
            attention_mask: array_from(&attention_mask),
            type_ids: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::e5;

    use super::*;

    // Encoder of "hello world" should produce ids [0, 33600, 31, 8999, 2]
    #[test]
    fn test_new() {
        let config = Config::new_unigram(e5().unwrap()).unwrap();
        let tokenizer = Tokenizer::new(&config).unwrap();
        let encoding = tokenizer.encode("hello world").unwrap();
        assert!(encoding.token_ids.shape() == &[1, 5]);
        assert!(encoding.token_ids == Array2::from_shape_vec((1, 5), vec![0, 33600, 31, 8999, 2]).unwrap());
    }
}
