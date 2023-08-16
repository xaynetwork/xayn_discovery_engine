# Copyright 2023 Xayn AG
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, version 3.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

from typing import List
from nltk.tokenize import sent_tokenize
from langchain.text_splitter import RecursiveCharacterTextSplitter, TextSplitter, NLTKTextSplitter
from transformers import PreTrainedTokenizerFast

tokenizer = PreTrainedTokenizerFast(tokenizer_file="tokenizer.json")

def token_len(s):
    return len(tokenizer(s).input_ids)

# like `langchain.text_splitter.NLTKTextSplitter` but with configurable language and not small split merging
class NLTKTextSplitter(TextSplitter):
    """Splitting text using NLTK package."""

    def __init__(self, *, separator="\n\n", language='german', **kwargs):
        """Initialize the NLTK splitter."""
        super().__init__(**kwargs)
        self._tokenizer = lambda x: sent_tokenize(x, language)
        self._separator = separator


    def split_text(self, text: str) -> List[str]:
        """Split incoming text and return chunks."""
        # First we naively split the large input into a bunch of smaller ones.
        splits = self._tokenizer(text)
        return self._merge_splits(splits, self._separator)

class TextSplitterWithBigChunkSplitter(TextSplitter):
    def __init__(self, *, primary: TextSplitter, secondary: TextSplitter, hard_chunks_size_limit: int):
        # setting chunk_size = hard_max_chunk is needed for using self._merge_splits
        super().__init__(chunk_size=hard_chunks_size_limit, chunk_overlap=0, length_function=token_len)
        self._primary = primary
        self._secondary = secondary
        self._hard_chunks_size_limit = hard_chunks_size_limit

    def split_text(self, text: str) -> List[str]:
        main_splits = self._primary.split_text(text)

        # remove snippets that are larger than hard_max_chunk
        no_big_splits = []
        for split in main_splits:
            if token_len(split) > self._hard_chunks_size_limit:
                secondary_splits = self._secondary.split_text(split)
                no_big_splits.extend(secondary_splits)
            else:
                no_big_splits.append(split)

        return self._merge_splits(no_big_splits, "\n")

class SnippetExtractor(TextSplitterWithBigChunkSplitter):
    def __init__(self, *, language, chunks_size, hard_chunks_size_limit):
        super().__init__(
            primary=NLTKTextSplitter(language=language, chunks_size=chunks_size, chunks_overlap = 0, length_function=token_len),
            secondary=RecursiveCharacterTextSplitter(chunks_size=chunks_size, chunk_overlap=0, length_function=token_len),
            hard_chunks_size_limit=hard_chunks_size_limit
        )
