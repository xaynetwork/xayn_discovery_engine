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

use std::borrow::Cow;

use sqlx::{Database, Encode, QueryBuilder, Type};

use crate::storage::Error;

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

pub(crate) trait SqlxSqliteErrorExt<V> {
    /// In case of a foreign key violation return given error.
    ///
    /// Converts other `sqlx::Error`s to `storage::Error`s.
    fn on_fk_violation(self, error: Error) -> Result<V, Error>;

    /// In case of a row not found error return given error.
    ///
    /// Converts other `sqlx::Error`s to `storage::Error`s.
    fn on_row_not_found(self, error: Error) -> Result<V, Error>;
}

impl<V> SqlxSqliteErrorExt<V> for Result<V, sqlx::Error> {
    fn on_fk_violation(self, error: Error) -> Result<V, Error> {
        if let Err(sqlx::Error::Database(db_err)) = &self {
            if db_err.code() == Some(Cow::Borrowed("787")) {
                return Err(error);
            }
        }
        self.map_err(Into::into)
    }

    fn on_row_not_found(self, error: Error) -> Result<V, Error> {
        if let Err(sqlx::Error::RowNotFound) = &self {
            Err(error)
        } else {
            self.map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{document, storage::sqlite::SqliteStorage};

    use super::*;

    #[tokio::test]
    async fn test_fk_violation_is_invalid_document() {
        let storage = SqliteStorage::connect("sqlite::memory:").await.unwrap();

        sqlx::query("CREATE TABLE Foo(x INTEGER PRIMARY KEY);")
            .execute(&storage.pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE Bar(x INTEGER PRIMARY KEY REFERENCES Foo(x));")
            .execute(&storage.pool)
            .await
            .unwrap();

        let document_id = document::Id::new();

        let res = sqlx::query("INSERT INTO Bar(x) VALUES (?);")
            .bind(10u32)
            .execute(&storage.pool)
            .await
            .on_fk_violation(Error::NoDocument(document_id));

        assert!(matches!(res, Err(Error::NoDocument(id)) if id == document_id));

        let res = sqlx::query("malformed;")
            .execute(&storage.pool)
            .await
            .on_fk_violation(Error::NoDocument(document_id));

        assert!(!matches!(res, Err(Error::NoDocument(_))));
    }
}
