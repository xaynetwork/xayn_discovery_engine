// Copyright 2023 Xayn AG
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

use std::future::Future;

use anyhow::bail;
use async_trait::async_trait;
use derive_more::{From, Into};
use sqlx::{Postgres, Transaction, Type};

use crate::Error;

/// Allows running migration like "once per tenant" tasks.
#[async_trait(?Send)]
pub(crate) trait ExternalMigrator {
    async fn run_migration_if_needed(
        &mut self,
        name: &str,
        exec: impl Future<Output = Result<(), Error>>,
    ) -> Result<(), Error>;
}

#[derive(From, Into)]
pub(crate) struct PgExternalMigrator {
    tx: Transaction<'static, Postgres>,
}

#[derive(Debug, Copy, Clone, PartialEq, Type)]
#[sqlx(type_name = "external_migration_state", rename_all = "snake_case")]
enum ExternalMigrationState {
    Pending,
    Succeeded,
    Failed,
}

impl ExternalMigrationState {
    fn from_result(result: &Result<(), Error>) -> (Self, Option<String>) {
        match result {
            Ok(()) => (ExternalMigrationState::Succeeded, None),
            Err(error) => (ExternalMigrationState::Failed, Some(error.to_string())),
        }
    }
}

#[async_trait(?Send)]
impl ExternalMigrator for PgExternalMigrator {
    async fn run_migration_if_needed(
        &mut self,
        name: &str,
        exec: impl Future<Output = Result<(), Error>>,
    ) -> Result<(), Error> {
        let res = sqlx::query(
            "INSERT INTO external_migration(name)
                VALUES ($1)
                ON CONFLICT DO NOTHING;",
        )
        .bind(name)
        .execute(&mut self.tx)
        .await?;

        if res.rows_affected() == 0 {
            let (existing,) = sqlx::query_as::<_, (ExternalMigrationState,)>(
                "SELECT state
                    FROM external_migration
                    WHERE name = $1
                    FOR UPDATE;",
            )
            .bind(name)
            .fetch_one(&mut self.tx)
            .await?;

            return match existing {
                ExternalMigrationState::Succeeded => Ok(()),
                ExternalMigrationState::Failed => {
                    bail!("previous external migration failed: {name}")
                }
                ExternalMigrationState::Pending => {
                    unreachable!(/*
                        'pending' only exist during ongoing transactions as we don't share the
                        transaction we run on with other tasks this should make it impossible to
                        observe this state
                    */);
                }
            };
        }

        let result = exec.await;

        let (state, error) = ExternalMigrationState::from_result(&result);
        sqlx::query(
            "UPDATE external_migration
            SET
                state = $1,
                end_time = CURRENT_TIMESTAMP,
                error = $2
            WHERE
                name = $3;",
        )
        .bind(state)
        .bind(error)
        .bind(name)
        .execute(&mut self.tx)
        .await?;

        result
    }
}
