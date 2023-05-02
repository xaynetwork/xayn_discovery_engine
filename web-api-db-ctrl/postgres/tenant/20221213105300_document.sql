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

-- WARNING: We use the existence of `public.document` to determine
--          if there is a legacy setup in `public`. If you rename or
--          delete this table do make sure you update the check.
CREATE TABLE IF NOT EXISTS document (
    document_id TEXT NOT NULL PRIMARY KEY
);

ALTER TABLE interaction
    ADD FOREIGN KEY (doc_id)
    REFERENCES document (document_id)
    ON DELETE CASCADE;
