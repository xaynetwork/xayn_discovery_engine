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

CREATE TABLE IF NOT EXISTS center_of_interest (
    coi_id UUID NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL,
    is_positive BOOLEAN NOT NULL,
    embedding FLOAT4[] NOT NULL,
    view_count INTEGER NOT NULL DEFAULT 0,
    view_time_ms BIGINT NOT NULL DEFAULT 0,
    last_view TIMESTAMPTZ NOT NULL DEFAULT Now()
);

CREATE INDEX IF NOT EXISTS idx_coi_by_user_id
    ON center_of_interest(user_id);
