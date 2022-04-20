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

//! FFI and logic bindings to `discovery_engine_core`.

#![deny(
    clippy::pedantic,
    clippy::future_not_send,
    noop_method_call,
    rust_2018_idioms,
    rust_2021_compatibility,
    unused_qualifications,
    unsafe_op_in_unsafe_fn
)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

pub mod async_bindings;
mod tracing;
pub mod types;

use ::tracing::{event, Level};
use rusqlite::{params, Connection, Result};
use xayn_discovery_engine_core::Engine;

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

#[async_bindgen::api(
    use xayn_discovery_engine_core::{
        document::{Document, HistoricDocument, TimeSpent, UserReacted},
        InitConfig,
        Market,
    };

    use crate::types::engine::SharedEngine;
)]
impl XaynDiscoveryEngineAsyncFfi {
    /// Initializes the engine.
    #[allow(clippy::box_vec)]
    pub async fn initialize(
        config: Box<InitConfig>,
        state: Option<Box<Vec<u8>>>,
        history: Box<Vec<HistoricDocument>>,
    ) -> Box<Result<SharedEngine, String>> {
        tracing::init_tracing();

        let path = &config.smbert_vocab.clone();
        let vocab_path = "assets/smbert_v0001/vocab.txt";
        let db_path = "discovery_engine.db3";
        let path = path.replace(vocab_path, db_path);

        Box::new(
            Engine::from_config(*config, state.as_deref().map(Vec::as_slice), &history)
                .await
                .map(|engine| tokio::sync::Mutex::new(engine).into())
                .map(|engine| {
                    event!(Level::INFO, "\n\n [SQLite] trying to open DB in memory\n\n");
                    event!(Level::INFO, "\n\n [SQLite] path: {}\n\n", path);

                    let conn = Connection::open(&path).expect("[SQLite] failed to open database");

                    // conn.execute_batch("PRAGMA key = '3e30431105262861ffad1d2ada34e82c';")
                    conn.pragma_update(None, "key", "3e30431105262861ffad1d2ada34e82c")
                        .expect("[SQLite] pragma key failes to be set");

                    event!(Level::INFO, "\n\n [SQLite] DB opened!!\n\n");

                    conn.execute(
                        "CREATE TABLE IF NOT EXISTS person (
                            id    INTEGER PRIMARY KEY,
                            name  TEXT NOT NULL,
                            data  BLOB
                        )",
                        [],
                    )
                    .expect("[SQLite] crate table failed");

                    let me = Person {
                        id: 0,
                        name: "Steven".to_string(),
                        data: None,
                    };

                    conn.execute(
                        "INSERT INTO person (name, data) VALUES (?1, ?2)",
                        params![me.name, me.data],
                    )
                    .expect("[SQLite] insert data into table failed");

                    let mut stmt = conn
                        .prepare("SELECT id, name, data FROM person")
                        .expect("[SQLite] prepare statement failed");

                    let person_iter = stmt
                        .query_map([], |row| {
                            Ok(Person {
                                id: row.get(0)?,
                                name: row.get(1)?,
                                data: row.get(2)?,
                            })
                        })
                        .expect("[SQLite] select query failed");

                    for person in person_iter {
                        event!(
                            Level::INFO,
                            "\n\n [SQLite] person from db: {:?}\n\n",
                            person.unwrap()
                        );
                    }

                    // return the engine
                    engine
                })
                .map_err(|error| error.to_string()),
        )
    }

    /// Serializes the engine.
    pub async fn serialize(engine: &SharedEngine) -> Box<Result<Vec<u8>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .serialize()
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the markets.
    #[allow(clippy::box_vec)]
    pub async fn set_markets(
        engine: &SharedEngine,
        markets: Box<Vec<Market>>,
        history: Box<Vec<HistoricDocument>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_markets(&history, *markets)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Gets feed documents.
    #[allow(clippy::box_vec)]
    pub async fn get_feed_documents(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        max_documents: u32,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .get_feed_documents(&history, max_documents)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes time spent.
    pub async fn time_spent(
        engine: &SharedEngine,
        time_spent: Box<TimeSpent>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .time_spent(time_spent.as_ref())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Processes user reaction.
    ///
    /// The history is only required for positive reactions.
    #[allow(clippy::box_vec)]
    pub async fn user_reacted(
        engine: &SharedEngine,
        history: Option<Box<Vec<HistoricDocument>>>,
        reacted: Box<UserReacted>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .user_reacted(history.as_deref().map(Vec::as_slice), reacted.as_ref())
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Perform an active search with the given query parameters.
    pub async fn active_search(
        engine: &SharedEngine,
        query: Box<String>,
        page: u32,
        page_size: u32,
    ) -> Box<Result<Vec<Document>, String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .active_search(query.as_ref(), page, page_size)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the trusted sources and updates the stacks based on that.
    #[allow(clippy::box_vec)]
    pub async fn set_trusted_sources(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<String>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_trusted_sources(&history, *sources)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Sets the excluded sources and updates the stacks based on that.
    #[allow(clippy::box_vec)]
    pub async fn set_excluded_sources(
        engine: &SharedEngine,
        history: Box<Vec<HistoricDocument>>,
        sources: Box<Vec<String>>,
    ) -> Box<Result<(), String>> {
        Box::new(
            engine
                .as_ref()
                .lock()
                .await
                .set_excluded_sources(&history, *sources)
                .await
                .map_err(|error| error.to_string()),
        )
    }

    /// Disposes the engine.
    pub async fn dispose(engine: Box<SharedEngine>) {
        engine.as_ref().as_ref().lock().await;
    }
}
