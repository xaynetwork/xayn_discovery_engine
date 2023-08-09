-- Copyright 2023 Xayn AG
--
-- This program is free software: you can redistribute it and/or modify
-- it under the terms of the GNU Affero General Public License as
-- published by the Free Software Foundation, version 3.
--
-- This program is distributed in the hope that it will be useful,
-- but WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
-- GNU Affero General Public License for more details.
--
-- You should have received a copy of the GNU Affero General Public License
-- along with this program.  If not, see <https://www.gnu.org/licenses/>.

CREATE TABLE snippet (
    document_id TEXT NOT NULL
        REFERENCES document(document_id) ON DELETE CASCADE,
    sub_id INTEGER NOT NULL,
    snippet TEXT NOT NULL,
    embedding FLOAT4[] NOT NULL,

    PRIMARY KEY (document_id, sub_id)
);

INSERT INTO snippet(document_id, sub_id, snippet, embedding)
    SELECT document_id, 0, snippet, embedding FROM document;

ALTER TABLE document RENAME COLUMN snippet TO original;
ALTER TABLE document DROP COLUMN embedding;
