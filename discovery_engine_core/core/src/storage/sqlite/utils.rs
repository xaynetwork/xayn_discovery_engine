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

//! Module containing sqlite specific utilities.

use std::borrow::Cow;

use crate::storage::Error;

pub(crate) trait SqlxSqliteResultExt<T> {
    /// In case of a foreign key violation return given error.
    ///
    /// Converts other `sqlx::Error`s to `storage::Error`s.
    fn on_fk_violation(self, error: Error) -> Result<T, Error>;

    /// In case of a row not found error return given error.
    ///
    /// Converts other `sqlx::Error`s to `storage::Error`s.
    fn on_row_not_found(self, error: Error) -> Result<T, Error>;
}

impl<T> SqlxSqliteResultExt<T> for Result<T, sqlx::Error> {
    fn on_fk_violation(self, error: Error) -> Result<T, Error> {
        if let Err(sqlx::Error::Database(db_err)) = &self {
            if db_err.code() == Some(Cow::Borrowed("787")) {
                return Err(error);
            }
        }
        self.map_err(Into::into)
    }

    fn on_row_not_found(self, error: Error) -> Result<T, Error> {
        if let Err(sqlx::Error::RowNotFound) = &self {
            Err(error)
        } else {
            self.map_err(Into::into)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{document, storage::sqlite::setup::create_connection_pool};

    use super::*;

    #[tokio::test]
    async fn test_fk_violation_is_invalid_document() {
        let pool = create_connection_pool(None).await.unwrap();

        sqlx::query("CREATE TABLE Foo(x INTEGER PRIMARY KEY);")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("CREATE TABLE Bar(x INTEGER PRIMARY KEY REFERENCES Foo(x));")
            .execute(&pool)
            .await
            .unwrap();

        let document_id = document::Id::new();

        let res = sqlx::query("INSERT INTO Bar(x) VALUES (?);")
            .bind(10u32)
            .execute(&pool)
            .await
            .on_fk_violation(Error::NoDocument(document_id));

        assert!(matches!(res, Err(Error::NoDocument(id)) if id == document_id));

        let res = sqlx::query("malformed;")
            .execute(&pool)
            .await
            .on_fk_violation(Error::NoDocument(document_id));

        assert!(!matches!(res, Err(Error::NoDocument(_))));
    }
}
