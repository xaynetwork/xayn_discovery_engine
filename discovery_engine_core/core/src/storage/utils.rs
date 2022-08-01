// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Module containing non-database specific sqlx utilities.

use sqlx::{Database, Encode, QueryBuilder, Type};

pub(super) trait SqlxPushTupleExt<'args, DB: Database> {
    fn push_tuple<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: 'args + Encode<'args, DB> + Send + Type<DB>;
}

impl<'args, DB> SqlxPushTupleExt<'args, DB> for QueryBuilder<'args, DB>
where
    DB: Database,
{
    fn push_tuple<I>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator,
        I::Item: 'args + Encode<'args, DB> + Send + Type<DB>,
    {
        let mut separated = self.push("(").separated(", ");
        for value in values {
            separated.push_bind(value);
        }
        separated.push_unseparated(")");
        self
    }
}
