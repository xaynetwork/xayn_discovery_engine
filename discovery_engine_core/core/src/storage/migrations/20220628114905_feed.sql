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

CREATE TABLE IF NOT EXISTS FeedDocument (
    documentId BLOB NOT NULL
        PRIMARY KEY
        REFERENCES HistoricDocument(documentId) ON DELETE CASCADE
);

-- ordering of documents based on when they have
-- been first presented to the app (user)
CREATE TABLE IF NOT EXISTS PresentationOrdering(
    documentId BLOB NOT NULL
        PRIMARY KEY
        REFERENCES Document(id) ON DELETE CASCADE,
    -- unix epoch timestamp in seconds
    -- you can't use DEFAULT as it must be the same
    -- for all documents added in the same batch
    timestamp INTEGER NOT NULL,
    -- index in the batch of document which where
    -- presented to the app (user) at the same time
    inBatchIndex INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_presentation_ordering_sort
  ON PresentationOrdering(timestamp, inBatchIndex);

CREATE TABLE IF NOT EXISTS UserReaction (
    documentId BLOB NOT NULL
        PRIMARY KEY
        REFERENCES HistoricDocument(documentId) ON DELETE CASCADE,

    userReaction INTEGER NOT NULL DEFAULT 0
);
