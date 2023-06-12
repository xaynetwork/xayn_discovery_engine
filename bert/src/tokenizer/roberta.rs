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
use tract_onnx::prelude::{tvec, TValue, TVec};

use crate::{
    config::Config,
    tokenizer::{tvalue_from, Encoding, Tokenize},
};

/// A pre-configured Roberta tokenizer.
pub struct Tokenizer {
    roberta: TokenizerImpl<Unigram, Precompiled, Sequence, TemplateProcessing, Metaspace>,
    special_token_ids: [u32; 4],
}

impl Tokenizer {
    fn to_input(encoding: &tokenizers::Encoding) -> TVec<TValue> {
        tvec![
            tvalue_from(encoding.get_ids()),
            tvalue_from(encoding.get_attention_mask()),
        ]
    }
}

impl Tokenize for Tokenizer {
    fn new<P>(config: &Config<Self, P>) -> Result<Self, Error> {
        let vocab = serde_json::from_reader::<_, Vec<(String, f64)>>(BufReader::new(File::open(
            config.dir.join("vocab.txt"),
        )?))?;
        let unknown_token = config.extract::<String>("tokenizer.tokens.unknown")?;
        let unknown_id = vocab.iter().position(|(word, _)| word == &unknown_token);
        let model = Unigram::from(vocab, unknown_id)?;
        let unknown_id = model
            .token_to_id(&unknown_token)
            .ok_or("missing unknown token")?;

        // https://github.com/huggingface/spm_precompiled
        let normalizer = serde_json::from_str(&read_to_string(
            config
                .dir
                .join(config.extract::<String>("tokenizer.normalizer")?),
        )?)?;

        let continuation_token = config.extract("tokenizer.tokens.continuation")?;
        let pre_tokenizer = Sequence::new(vec![
            PreTokenizerWrapper::WhitespaceSplit(WhitespaceSplit),
            PreTokenizerWrapper::Metaspace(Metaspace::new(continuation_token, true)),
        ]);

        let class_token = config.extract::<String>("tokenizer.tokens.class")?;
        let class_id = model.token_to_id(&class_token).ok_or("missing cls token")?;
        let separation_token = config.extract::<String>("tokenizer.tokens.separation")?;
        let separation_id = model
            .token_to_id(&separation_token)
            .ok_or("missing sep token")?;
        let post_processor = TemplateProcessingBuilder::default()
            .try_single(format!("{class_token}:0 $A:0 {separation_token}:0"))?
            .try_pair(format!(
                "{class_token}:0 $A:0 {separation_token}:0 {separation_token}:0 $B:0 {separation_token}:0"
            ))?
            .special_tokens(vec![(class_token, class_id), (separation_token, separation_id)])
            .build()?;

        let padding_token = config.extract::<String>("tokenizer.tokens.padding")?;
        let padding_id = model
            .token_to_id(&padding_token)
            .ok_or("missing pad token")?;
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

        let roberta = TokenizerBuilder::new()
            .with_model(model)
            .with_normalizer(Some(normalizer))
            .with_pre_tokenizer(Some(pre_tokenizer))
            .with_post_processor(Some(post_processor))
            .with_padding(Some(padding))
            .with_truncation(Some(truncation))
            .build()?;
        let special_token_ids = [unknown_id, class_id, separation_id, padding_id];

        Ok(Tokenizer {
            roberta,
            special_token_ids,
        })
    }

    fn encode(&self, sequence: impl AsRef<str>) -> Result<Encoding, Error> {
        Ok(Encoding {
            encoding: self.roberta.encode(sequence.as_ref(), true)?,
            to_input: Self::to_input,
        })
    }

    fn special_token_ids(&self) -> &[u32] {
        &self.special_token_ids
    }
}

#[cfg(test)]
mod tests {
    use xayn_test_utils::asset::smroberta;

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
        let tokenizer = tokenizer(42);
        assert!(tokenizer.roberta.get_normalizer().is_some());
        assert!(tokenizer.roberta.get_pre_tokenizer().is_some());
        assert!(tokenizer.roberta.get_post_processor().is_some());
        assert!(tokenizer.roberta.get_padding().is_some());
        assert!(tokenizer.roberta.get_truncation().is_some());
        assert!(tokenizer.roberta.get_decoder().is_none());
    }
}
