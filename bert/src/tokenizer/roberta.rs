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

use std::fs::read_to_string;

use csv::ReaderBuilder;
use itertools::Itertools;
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

/// A pre-configured Roberta tokenizer.
pub struct Tokenizer(TokenizerImpl<Unigram, Precompiled, Sequence, TemplateProcessing, Metaspace>);

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let vocab = ReaderBuilder::new()
            .has_headers(false)
            .escape(Some(b'\\'))
            .from_path(config.dir.join("vocab.txt"))?
            .into_deserialize::<(String, f64)>()
            .try_collect::<_, Vec<_>, _>()?;
        let unk_token = config.extract::<String>("tokenizer.tokens.unknown")?;
        let unk_id = vocab.iter().position(|(word, _)| word == &unk_token);
        let model = Unigram::from(vocab, unk_id)?;

        // https://github.com/huggingface/spm_precompiled
        let normalizer = serde_json::from_str(&read_to_string(
            config
                .dir
                .join(config.extract::<String>("tokenizer.normalizer")?),
        )?)?;

        let cont_token = config.extract("tokenizer.tokens.continuation")?;
        let pre_tokenizer = Sequence::new(vec![
            PreTokenizerWrapper::WhitespaceSplit(WhitespaceSplit),
            PreTokenizerWrapper::Metaspace(Metaspace::new(cont_token, true)),
        ]);

        let cls_token = config.extract::<String>("tokenizer.tokens.class")?;
        let cls_id = model.token_to_id(&cls_token).ok_or("missing cls token")?;
        let sep_token = config.extract::<String>("tokenizer.tokens.separation")?;
        let sep_id = model.token_to_id(&sep_token).ok_or("missing sep token")?;
        let post_processor = TemplateProcessingBuilder::default()
            .try_single(format!("{cls_token}:0 $A:0 {sep_token}:0"))?
            .try_pair(format!(
                "{cls_token}:0 $A:0 {sep_token}:0 {sep_token}:0 $B:0 {sep_token}:0"
            ))?
            .special_tokens(vec![(cls_token, cls_id), (sep_token, sep_id)])
            .build()?;

        let pad_token = config.extract::<String>("tokenizer.tokens.padding")?;
        let padding = PaddingParams {
            strategy: PaddingStrategy::Fixed(config.token_size),
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: model.token_to_id(&pad_token).ok_or("missing pad token")?,
            pad_type_id: 0,
            pad_token,
        };
        let truncation = TruncationParams {
            direction: TruncationDirection::Right,
            max_length: config.token_size,
            strategy: TruncationStrategy::LongestFirst,
            stride: 0,
        };

        let decoder = Metaspace::new(cont_token, true);

        TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(normalizer))
            .with_pre_tokenizer(Some(pre_tokenizer))
            .with_post_processor(Some(post_processor))
            .with_padding(Some(padding))
            .with_truncation(Some(truncation))
            .with_decoder(Some(decoder))
            .build()
            .map(Tokenizer)
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        let encoding = self.0.encode(sequence.as_ref(), true)?;
        let array_from =
            |slice: &[u32]| Array2::from_shape_fn((1, slice.len()), |(_, i)| i64::from(slice[i]));

        Ok(Encoding {
            token_ids: array_from(encoding.get_ids()),
            attention_mask: array_from(encoding.get_attention_mask()),
            type_ids: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use xayn_ai_test_utils::asset::smroberta;

    use super::*;

    fn tokenizer(token_size: usize) -> Tokenizer {
        let config = Config::new(smroberta().unwrap())
            .unwrap()
            .with_token_size(token_size)
            .unwrap()
            .with_tokenizer();
        Tokenizer::new(&config).unwrap()
    }

    #[test]
    fn test_new() {
        let tok = tokenizer(42);
        assert!(tok.0.get_normalizer().is_some());
        assert!(tok.0.get_pre_tokenizer().is_some());
        assert!(tok.0.get_post_processor().is_some());
        assert!(tok.0.get_padding().is_some());
        assert!(tok.0.get_truncation().is_some());
        assert!(tok.0.get_decoder().is_some());
    }
}
