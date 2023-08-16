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

use futures_util::{stream::FuturesOrdered, TryStreamExt};
use xayn_snippet_extractor::{self as snippet_extractor, SnippetExtractor};
use xayn_summarizer::{self as summarizer, summarize, Source, Summarizer};

use crate::{
    embedding::Embedder,
    error::common::InvalidDocumentSnippet,
    models::{DocumentContent, DocumentSnippet, PreprocessingStep},
    Error,
};

pub(super) async fn preprocess_document(
    embedder: &Embedder,
    original: DocumentSnippet,
    preprocessing_step: &mut PreprocessingStep,
) -> Result<Vec<DocumentContent>, Error> {
    Ok(match *preprocessing_step {
        PreprocessingStep::None => embed_whole(embedder, original).await?,
        PreprocessingStep::Summarize => embed_with_summarizer(embedder, original).await?,
        #[allow(deprecated)]
        PreprocessingStep::CuttersSplit | PreprocessingStep::NltkSplitV1 => {
            *preprocessing_step = PreprocessingStep::NltkSplitV1;
            embed_with_nltk(embedder, &original).await?
        }
    })
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

async fn embed_with_nltk(
    embedder: &Embedder,
    snippet: &DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error> {
    let extractor = SnippetExtractor::initialize(&snippet_extractor::Config::default())?;
    let snippets = extractor
        .run(snippet)?
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
