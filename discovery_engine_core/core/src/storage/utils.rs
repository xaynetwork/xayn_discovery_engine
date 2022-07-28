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

use crate::{document, storage::Error};

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

trait SqlxSqliteErrorExt<V> {
    /// Use this on the result of a sqlite query which might have failed a foreign
    /// key constraint when an illegal document was passed in.
    ///
    /// For example it can be used in `.user_reacted` to handle it being
    /// called with a random document id.
    ///
    /// If there is an error and it is a fk violation an appropriate error variant is
    /// used instead of the default generic database error.
    fn fk_violation_is_invalid_document_id(self, id: document::Id) -> Result<V, Error>;
}

impl<V> SqlxSqliteErrorExt<V> for Result<V, sqlx::Error> {
    fn fk_violation_is_invalid_document_id(self, id: document::Id) -> Result<V, Error> {
        if let Err(sqlx::Error::Database(db_err)) = &self {
            if db_err.code() == Some(Cow::Borrowed("787")) {
                return Err(Error::InvalidDocumentId(id));
            }
        }
        self.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::sqlite::SqliteStorage;

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
            .bind(10)
            .execute(&storage.pool)
            .await
            .fk_violation_is_invalid_document_id(document_id);

        assert!(matches!(res, Err(Error::InvalidDocumentId(id)) if id == document_id));

        let res = sqlx::query("malformed;")
            .execute(&storage.pool)
            .await
            .fk_violation_is_invalid_document_id(document_id);

        assert!(!matches!(res, Err(Error::InvalidDocumentId(_))));
    }
}
