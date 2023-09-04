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

use std::{io, time::Duration};

pub use deadpool::unmanaged::PoolError;
use deadpool::{
    unmanaged::{Object, Pool, PoolConfig},
    Runtime,
};
use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;
use xayn_web_api_shared::serde::serde_duration_in_config;

use crate::{Error, SnippetExtractor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    threads_per_cpu: f32,
    #[serde(with = "serde_duration_in_config")]
    acquisition_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threads_per_cpu: 1.0,
            acquisition_timeout: Duration::from_secs(15),
        }
    }
}

pub struct SnippetExtractorPool {
    pool: Pool<SnippetExtractor>,
}

impl SnippetExtractorPool {
    #[allow(clippy::missing_panics_doc)]
    pub fn new(config: &super::Config) -> Result<Self, Error> {
        let num_cpus = num_cpus::get();
        let max_size = (num_cpus as f32 * config.pool.threads_per_cpu)
            .ceil()
            .max(1.0) as usize;
        let pool = Pool::from_config(&PoolConfig {
            max_size,
            timeout: Some(config.pool.acquisition_timeout),
            runtime: Some(Runtime::Tokio1),
        });

        for _ in 0..num_cpus {
            let extractor = SnippetExtractor::new(config.clone())?;
            pool.try_add(extractor).map_err(|(_, err)| err).unwrap(/* can't happen */);
        }
        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<PooledSnippetExtractor, PoolError> {
        self.pool.get().await.map(PooledSnippetExtractor)
    }
}

#[derive(Deref, DerefMut)]
pub struct PooledSnippetExtractor(Object<SnippetExtractor>);

impl PooledSnippetExtractor {
    pub async fn extract_snippet(
        mut self,
        tokenizer: String,
        document: String,
    ) -> Result<Vec<String>, Error> {
        spawn_blocking(move || self.0.extract_snippet(&tokenizer, &document))
            .await
            .map_err(|join_error| Error::Io(io::Error::new(io::ErrorKind::Other, join_error)))?
    }
}
