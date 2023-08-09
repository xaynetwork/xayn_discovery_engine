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

ALTER TABLE interaction
    RENAME COLUMN doc_id TO document_id;

ALTER TABLE interaction
    ADD COLUMN sub_id INTEGER NOT NULL DEFAULT 0,
    DROP CONSTRAINT interaction_pkey,
    ADD PRIMARY KEY (document_id, sub_id, user_id, time_stamp),
    ADD FOREIGN KEY (document_id, sub_id) REFERENCES snippet(document_id, sub_id) ON DELETE CASCADE;
