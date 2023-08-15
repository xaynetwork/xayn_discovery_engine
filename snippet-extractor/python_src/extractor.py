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

from collections.abc import Sequence
from langchain.text_splitter import RecursiveCharacterTextSplitter

def utf8_length(x: str) -> int:
    return len(x.encode('utf-8'));

def extract_snippets(*, document: str, chunk_size: int, chunk_overlap: int) -> Sequence[str]:
    text_splitter = RecursiveCharacterTextSplitter(
        chunk_size = chunk_size,
        chunk_overlap  = chunk_overlap,
        length_function = utf8_length,
        add_start_index = True,
    )
    return text_splitter.split_text(document)

