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

CREATE TYPE indexed_property_type AS ENUM (
    'bool',
    'number',
    'string',
    'string[]',
    'date'
);

CREATE TABLE indexed_property_definition (
    name TEXT NOT NULL PRIMARY KEY,
    type indexed_property_type NOT NULL
);

INSERT INTO indexed_property_definition(name, type)
    VALUES ('publication_date', 'date')
