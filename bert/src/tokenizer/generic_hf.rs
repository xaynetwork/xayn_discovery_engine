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

use ndarray::Array2;
use tokenizers::{
    Error,
    tokenizer::Tokenizer as HfTokenizer
};

use crate::{
    config::Config,
    tokenizer::{Encoding, Tokenize},
};

/// A pre-configured E5 tokenizer.
pub struct Tokenizer{
    hf_tokenizer: HfTokenizer,
}

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let tokenizer = HfTokenizer::from_file(config.dir.join("tokenizer.json"))?;
        Ok(Tokenizer{hf_tokenizer: tokenizer})
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        let tokens = self.hf_tokenizer.encode(sequence.as_ref(), true)?;
        let token_ids: Vec<u32> = tokens.get_ids().to_vec();
        let attention_mask = vec![1; token_ids.len()];
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
        assert!(encoding.token_ids.shape() == [1, 5]);
        assert!(encoding.token_ids == Array2::from_shape_vec((1, 5), vec![0, 33600, 31, 8999, 2]).unwrap());
    }
}
