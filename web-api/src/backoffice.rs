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

pub(crate) mod preprocessor;
pub(crate) mod routes;

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{app::SetupError, storage::elastic::IndexUpdateConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default)]
#[cfg_attr(test, serde(deny_unknown_fields))]
pub struct IngestionConfig {
    pub(crate) max_document_batch_size: usize,
    pub(crate) max_indexed_properties: usize,
    pub(crate) index_update: IndexUpdateConfig,
    pub(crate) max_snippet_size: usize,
    pub(crate) max_properties_size: usize,
    pub(crate) max_properties_string_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            max_document_batch_size: 100,
            // 10 + publication_date
            max_indexed_properties: 11,
            index_update: IndexUpdateConfig::default(),
            max_snippet_size: 2_048,
            max_properties_size: 2_560,
            max_properties_string_size: 2_048,
        }
    }
}

impl IngestionConfig {
    pub(crate) fn validate(&self) -> Result<(), SetupError> {
        if self.max_indexed_properties == 0 {
            bail!("invalid IngestionConfig, max_indexed_properties must be > 0 to account for publication_date");
        }
        self.index_update.validate()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_ingestion_config() {
        IngestionConfig::default().validate().unwrap();
    }
}
