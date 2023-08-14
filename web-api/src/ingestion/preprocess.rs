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

use xayn_ai_bert::NormalizedEmbedding;
use xayn_summarizer::{summarize, Config, Source, Summarizer};

use crate::{
    embedding::Embedder,
    models::{DocumentSnippet, PreprocessingStep},
    Error,
};

pub(super) type DocumentContent = (DocumentSnippet, NormalizedEmbedding);

pub(super) fn preprocess_document(
    embedder: &Embedder,
    raw_text: DocumentSnippet,
    preprocessing_step: PreprocessingStep,
) -> Result<DocumentContent, Error> {
    Ok(match preprocessing_step {
        PreprocessingStep::None => embed_whole(embedder, raw_text)?,
        PreprocessingStep::Summarize => embed_with_summarizer(embedder, raw_text)?,
    })
}

fn embed_whole(embedder: &Embedder, snippet: DocumentSnippet) -> Result<DocumentContent, Error> {
    let embedding = embedder.run(&snippet)?;
    Ok((snippet, embedding))
}

fn embed_with_summarizer(
    embedder: &Embedder,
    snippet: DocumentSnippet,
) -> Result<DocumentContent, Error> {
    let summary = summarize(
        &Summarizer::Naive,
        &Source::PlainText {
            text: snippet.to_string(),
        },
        &Config::default(),
    );
    let embedding = embedder.run(&summary)?;
    Ok((snippet, embedding))
}
