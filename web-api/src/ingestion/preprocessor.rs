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

use std::future::Future;

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

pub(crate) async fn preprocess<Fun, Fut>(
    embedder: &Embedder,
    snippet_extractor: Fun,
    text_extractor: &TextExtractor,
    original: InputData,
    preprocessing_step: &mut PreprocessingStep,
) -> Result<Vec<DocumentContent>, PreprocessError>
where
    Fun: FnOnce() -> Fut,
    Fut: Future<Output = Result<PooledSnippetExtractor, Error>>,
{
    let original = match original {
        InputData::Snippet(snippet) => snippet,
        InputData::Binary(binary) => text_extractor.extract_text(binary).await?,
    };

    let res = match *preprocessing_step {
        PreprocessingStep::None => embed_whole(embedder, original).await,
        PreprocessingStep::Summarize => embed_with_summarizer(embedder, original).await,
        PreprocessingStep::CuttersSplit | PreprocessingStep::NltkSplitV1 => {
            *preprocessing_step = PreprocessingStep::NltkSplitV1;
            embed_with_nltk(embedder, snippet_extractor, original).await
        }
    };

    res.map_err(PreprocessError::Fatal)
}

async fn embed_whole(
    embedder: &Embedder,
    snippet: DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error> {
    let embedding = embedder.run(&snippet).await?;
    Ok(vec![DocumentContent { snippet, embedding }])
}

async fn embed_with_summarizer(
    embedder: &Embedder,
    snippet: DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error> {
    let summary = summarize(
        &Summarizer::Naive,
        &Source::PlainText {
            text: snippet.to_string(),
        },
        &summarizer::Config::default(),
    );
    let embedding = embedder.run(&summary).await?;
    Ok(vec![DocumentContent {
        // Hint: Yes we do not use the summary, this is so that keyword/text search
        //       can use the original text.
        snippet,
        embedding,
    }])
}

async fn embed_with_nltk<Fun, Fut>(
    embedder: &Embedder,
    snippet_extractor: Fun,
    snippet: DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error>
where
    Fun: FnOnce() -> Fut,
    Fut: Future<Output = Result<PooledSnippetExtractor, Error>>,
{
    let snippets = snippet_extractor()
        .await?
        .extract_snippet("default".into(), snippet.into())
        .await?;

    let snippets = snippets
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
