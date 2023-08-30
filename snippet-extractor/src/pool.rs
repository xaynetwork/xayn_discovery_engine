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

use std::time::Duration;

use deadpool::{
    unmanaged::{Object, Pool, PoolConfig},
    Runtime,
};

use crate::{Config, Error, SnippetExtractor};

pub type PooledSnippetExtractor = Object<SnippetExtractor>;
pub use deadpool::unmanaged::PoolError;

#[derive(Clone)]
pub struct SnippetExtractorPool {
    // Hint: Internally `Arc`ed
    pool: Pool<SnippetExtractor>,
}

impl SnippetExtractorPool {
    #[allow(clippy::missing_panics_doc)]
    pub fn new(config: &Config) -> Result<Self, Error> {
        let num_cpus = num_cpus::get();
        let pool = Pool::from_config(&PoolConfig {
            max_size: num_cpus,
            // TODO[pmk/now] decide value based on whole request timeout and make configurable
            timeout: Some(Duration::from_secs(15)),
            runtime: Some(Runtime::Tokio1),
        });

        for _ in 0..num_cpus {
            let extractor = SnippetExtractor::new(config.clone())?;
            pool.try_add(extractor).map_err(|(_, err)| err).unwrap(/* can't happen */);
        }
        Ok(Self { pool })
    }

    pub async fn get(&self) -> Result<PooledSnippetExtractor, PoolError> {
        self.pool.get().await
    }
}
