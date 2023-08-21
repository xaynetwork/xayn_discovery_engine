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

use displaydoc::Display;
use futures_util::{stream::FuturesOrdered, TryStreamExt};
use thiserror::Error;
use xayn_snippet_extractor::pool::PooledSnippetExtractor;
use xayn_summarizer::{self as summarizer, summarize, Source, Summarizer};

use super::routes::InputData;
use crate::{
    embedding::Embedder,
    error::common::InvalidDocumentSnippet,
    extractor::TextExtractor,
    models::{DocumentContent, DocumentSnippet, PreprocessingStep},
    Error,
};

#[derive(Error, Debug, Display)]
pub(crate) enum PreprocessError {
    /// Fatal error
    Fatal(Error),
    /// Invalid request
    Invalid(Error),
}

pub(crate) struct Preprocessor<'a> {
    embedder: &'a Embedder,
    snippet_extractor: PooledSnippetExtractor,
    text_extractor: &'a TextExtractor,
}

impl<'a> Preprocessor<'a> {
    pub(crate) fn new(
        embedder: &'a Embedder,
        snippet_extractor: PooledSnippetExtractor,
        text_extractor: &'a TextExtractor,
    ) -> Self {
        Self {
            embedder,
            snippet_extractor,
            text_extractor,
        }
    }

    pub(crate) async fn preprocess(
        self,
        original: InputData,
        preprocessing_step: &mut PreprocessingStep,
    ) -> Result<Vec<DocumentContent>, PreprocessError> {
        let original = match original {
            InputData::Snippet(snippet) => snippet,
            InputData::Binary(binary) => self.text_extractor.extract_text(binary).await?,
        };

        let res = match *preprocessing_step {
            PreprocessingStep::None => self.embed_whole(original).await,
            PreprocessingStep::Summarize => self.embed_with_summarizer(original).await,
            PreprocessingStep::CuttersSplit | PreprocessingStep::NltkSplitV1 => {
                *preprocessing_step = PreprocessingStep::NltkSplitV1;
                self.embed_with_nltk(original).await
            }
        };

        res.map_err(PreprocessError::Fatal)
    }

    async fn embed_whole(&self, snippet: DocumentSnippet) -> Result<Vec<DocumentContent>, Error> {
        let embedding = self.embedder.run(&snippet).await?;
        Ok(vec![DocumentContent { snippet, embedding }])
    }

    async fn embed_with_summarizer(
        &self,
        snippet: DocumentSnippet,
    ) -> Result<Vec<DocumentContent>, Error> {
        let summary = summarize(
            &Summarizer::Naive,
            &Source::PlainText {
                text: snippet.to_string(),
            },
            &summarizer::Config::default(),
        );
        let embedding = self.embedder.run(&summary).await?;
        Ok(vec![DocumentContent {
            // Hint: Yes we do not use the summary, this is so that keyword/text search
            //       can use the original text.
            snippet,
            embedding,
        }])
    }

    async fn embed_with_nltk(
        self,
        snippet: DocumentSnippet,
    ) -> Result<Vec<DocumentContent>, Error> {
        let embedder = &self.embedder;
        let snippets = self
            .snippet_extractor
            .extract_snippet("default".into(), snippet.into())
            .await?
            .into_iter()
            .map(|split| async move {
                let snippet = DocumentSnippet::new(split, usize::MAX)?;
                let embedding = embedder.run(&snippet).await?;
                Ok::<_, Error>(DocumentContent { snippet, embedding })
            })
            .collect::<FuturesOrdered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        if snippets.is_empty() {
            Err(InvalidDocumentSnippet::NoSnippets {}.into())
        } else {
            Ok(snippets)
        }
    }
}
