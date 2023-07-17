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

CREATE TYPE preprocessing_step AS ENUM (
    'none',
    'split',
    'summarize'
);

ALTER TABLE document
    ALTER COLUMN is_summarized DROP DEFAULT,
    ALTER COLUMN is_summarized TYPE preprocessing_step
        USING CASE
            WHEN is_summarized THEN 'summarize'::preprocessing_step
            ELSE 'none'::preprocessing_step
        END,
    ALTER COLUMN is_summarized SET DEFAULT 'none';

ALTER TABLE document
    RENAME COLUMN is_summarized TO preprocessing_step;
