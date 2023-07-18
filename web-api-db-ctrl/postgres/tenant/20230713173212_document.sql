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

ALTER TABLE document
    -- Hint:    Due to limitations in sqlx it's currently impractical to
    --          to have a FLOAT4[][] type. The jsonb[] array also has the
    --          benefit of pairing the embedding with it's metadata.
    --
    -- The implicit json schema is `{ "embedding": [...], "range": [<start>, <end>]  }`
    -- with `start` & `end` being utf8 byte offsets into the original document. If
    -- `range` is missing it's implies a full range.
    ALTER COLUMN embedding TYPE jsonb[]
        USING ARRAY[json_build_object('embedding',  array_to_json(embedding))];

ALTER TABLE document RENAME COLUMN embedding TO embeddings;
