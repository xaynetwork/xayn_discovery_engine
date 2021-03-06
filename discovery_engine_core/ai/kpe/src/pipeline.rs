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

use displaydoc::Display;
use thiserror::Error;
use xayn_discovery_engine_layer::io::LoadingBinParamsFailed;

use crate::{
    model::{bert::Bert, classifier::Classifier, cnn::Cnn, ModelError},
    tokenizer::{key_phrase::RankedKeyPhrases, Tokenizer, TokenizerError},
};

/// A pipeline for a KPE model.
///
/// Can be built from a [`Config`] and consists of a tokenizer, a Bert model, a CNN model and a
/// Classifier model.
///
/// [`Config`]: crate::config::Config
pub struct Pipeline {
    pub(crate) tokenizer: Tokenizer<{ Cnn::KEY_PHRASE_SIZE }>,
    pub(crate) bert: Bert,
    pub(crate) cnn: Cnn,
    pub(crate) classifier: Classifier,
}

/// The potential errors of the [`Pipeline`].
#[derive(Debug, Display, Error)]
pub enum PipelineError {
    /// Failed to run the tokenizer: {0}
    Tokenizer(#[from] TokenizerError),
    /// Failed to run the model: {0}
    Model(#[from] ModelError),
    /// Failed to load binary parameters from a file: {0}
    BinParams(#[from] LoadingBinParamsFailed),
    /// Failed to build the model: {0}
    ModelBuild(#[source] ModelError),
}

impl Pipeline {
    /// Extracts the key phrases from the sequence ranked in descending order.
    pub fn run(&self, sequence: impl AsRef<str>) -> Result<RankedKeyPhrases, PipelineError> {
        let (encoding, key_phrases) = self
            .tokenizer
            .encode(sequence)
            .ok_or(ModelError::NotEnoughWords)?;
        let embeddings = self.bert.run(encoding.token_ids, encoding.attention_mask)?;
        let features = self.cnn.run(&embeddings, &encoding.valid_mask)?;
        let scores = self.classifier.run(&features, &encoding.active_mask);

        Ok(key_phrases.rank(scores))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, error::Error};

    use xayn_discovery_engine_test_utils::kpe::{bert, classifier, cnn, vocab};
    use xayn_discovery_engine_tokenizer::CaseChars;

    use crate::config::Config;

    #[test]
    fn test_run_unique() -> Result<(), Box<dyn Error>> {
        let kpe = Config::from_files(vocab()?, bert()?, cnn()?, classifier()?)?
            .with_token_size(8)?
            .with_case(CaseChars::Keep)
            .build()?;

        let actual = kpe.run("A b c d e.")?.0.into_iter().collect::<HashSet<_>>();
        let expected = [
            "a",
            "b",
            "c",
            "d",
            "e.",
            "a b",
            "b c",
            "c d",
            "d e.",
            "a b c",
            "b c d",
            "c d e.",
            "a b c d",
            "b c d e.",
            "a b c d e.",
        ]
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_run_duplicate() -> Result<(), Box<dyn Error>> {
        let kpe = Config::from_files(vocab()?, bert()?, cnn()?, classifier()?)?
            .with_token_size(7)?
            .with_case(CaseChars::Keep)
            .build()?;

        let actual = kpe.run("A a A a A")?.0.into_iter().collect::<HashSet<_>>();
        let expected = ["a", "a a", "a a a", "a a a a", "a a a a a"]
            .iter()
            .map(ToString::to_string)
            .collect::<HashSet<_>>();
        assert_eq!(actual, expected);
        Ok(())
    }
}
