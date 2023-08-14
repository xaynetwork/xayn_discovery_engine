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

use xayn_summarizer::{summarize, Config, Source, Summarizer};

use crate::{
    embedding::Embedder,
    models::{DocumentContent, DocumentSnippet, PreprocessingStep},
    Error,
};

pub(super) fn preprocess_document(
    embedder: &Embedder,
    original: DocumentSnippet,
    preprocessing_step: PreprocessingStep,
) -> Result<Vec<DocumentContent>, Error> {
    Ok(match preprocessing_step {
        PreprocessingStep::None => embed_whole(embedder, original)?,
        PreprocessingStep::Summarize => embed_with_summarizer(embedder, original)?,
    })
}

fn embed_whole(
    embedder: &Embedder,
    snippet: DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error> {
    let embedding = embedder.run(&snippet)?;
    Ok(vec![DocumentContent { snippet, embedding }])
}

fn embed_with_summarizer(
    embedder: &Embedder,
    snippet: DocumentSnippet,
) -> Result<Vec<DocumentContent>, Error> {
    let summary = summarize(
        &Summarizer::Naive,
        &Source::PlainText {
            text: snippet.to_string(),
        },
        &Config::default(),
    );
    let embedding = embedder.run(&summary)?;
    Ok(vec![DocumentContent {
        // Hint: Yes we do not use the summary, this is so that keyword/text search
        //       can use the original text.
        snippet,
        embedding,
    }])
}
