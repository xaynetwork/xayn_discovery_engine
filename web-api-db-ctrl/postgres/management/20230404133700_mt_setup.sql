--  Copyright 2023 Xayn AG
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

-- This table is mainly used for being able to list tenants, which
-- is mainly needed to run transactions on all tenant schemas.
CREATE TABLE tenant (
    tenant_id TEXT PRIMARY KEY NOT NULL,
    is_legacy_tenant BOOLEAN NOT NULL DEFAULT false
);


CREATE UNIQUE INDEX only_one_legacy_tenant
    ON tenant (is_legacy_tenant)
    WHERE (is_legacy_tenant);
