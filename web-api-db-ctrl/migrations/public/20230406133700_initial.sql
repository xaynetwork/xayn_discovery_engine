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

REVOKE CREATE ON SCHEMA public FROM PUBLIC;

-- the user must be created elsewhere for practical reasons of us not having
-- a way to set/return the password
-- CREATE USER "web-api-mt" NOINHERIT LOGIN PASSWORD 'foobar';
--
-- "$user" is NOT a bind but a placeholder which will be replaced
-- with the current user/role. I.e. it automatically changes when
-- `SET ROLE` is used.
ALTER USER "web-api-mt" SET search_path TO "$user";
