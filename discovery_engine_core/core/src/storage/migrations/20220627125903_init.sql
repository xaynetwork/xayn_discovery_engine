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

CREATE TABLE IF NOT EXISTS Document (
    id BLOB NOT NULL
        PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS HistoricDocument (
    document BLOB NOT NULL
        PRIMARY KEY
        REFERENCES Document(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS NewsResource (
    document BLOB NOT NULL
        PRIMARY KEY
        REFERENCES Document(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    snippet TEXT NOT NULL,
    topic TEXT NOT NULL,
    url TEXT NOT NULL,
    image TEXT,
    -- format should be RFC3339/ISO8601 & sqlite compliant
    datePublished TEXT NOT NULL,
    -- implied by url, but allows us to easier implement
    -- things like pruning history when excluding a source
    source TEXT NOT NULL,
    -- compound format <2-letter-lang><2-letter-state>
    -- should be same as market primary key
    -- but for now it can't be a foreign key
    market TEXT
);

CREATE TABLE IF NOT EXISTS NewscatcherData (
    document BLOB NOT NULL
        PRIMARY KEY
        REFERENCES NewsResource(document) ON DELETE CASCADE,
    domainRank INTEGER NOT NULL,
    score REAL NOT NULL
);
