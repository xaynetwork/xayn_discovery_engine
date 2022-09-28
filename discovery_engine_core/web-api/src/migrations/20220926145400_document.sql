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

CREATE TABLE IF NOT EXISTS interaction (
    doc_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    time_stamp TIMESTAMPTZ NOT NULL DEFAULT Now(),
    user_reaction SMALLINT NOT NULL,
    PRIMARY KEY (doc_id, user_id, time_stamp)
);

CREATE INDEX IF NOT EXISTS idx_interaction_by_user_id
    ON interaction(user_id);
