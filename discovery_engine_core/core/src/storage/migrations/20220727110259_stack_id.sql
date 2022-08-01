--  Copyright 2022 Xayn AG
--
--  This program is free software: you can redistribute it and/or modify
--  it under the terms of the GNU Affero General Public License as
--  published by the Free Software Foundation, version 3.
--
--  This program is distributed in the hope that it will be useful,
--  but WITHOUT ANY WARRANTY; without even the implied warranty of
--  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
--  GNU Affero General Public License for more details.
--
--  You should have received a copy of the GNU Affero General Public License
--  along with this program.  If not, see <https://www.gnu.org/licenses/>.

CREATE TABLE IF NOT EXISTS Stack(
    stackId BLOB NOT NULL PRIMARY KEY
    -- additional fields will be added when the serialized state is turned into the db
);

CREATE TABLE IF NOT EXISTS StackDocument(
    documentId BLOB NOT NULL
        PRIMARY KEY
        REFERENCES Document(id) ON DELETE CASCADE,
    stackId BLOB NOT NULL
        REFERENCES Stack(stackId) ON DELETE CASCADE
    -- additional fields will be added when the serialized state is turned into the db
);

CREATE INDEX IF NOT EXISTS idx_stack_documents_by_stack
  ON StackDocument(stackId);
