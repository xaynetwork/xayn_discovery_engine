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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use xayn_ai_coi::CoiConfig;

use crate::personalization::PersonalizationConfig;

#[derive(Clone, Debug, Serialize)]
pub(super) struct StateConfig {
    pub(super) coi: CoiConfig,
    pub(super) personalization: PersonalizationConfig,
    pub(super) time: DateTime<Utc>,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            coi: CoiConfig::default(),
            personalization: PersonalizationConfig::default(),
            time: Utc::now(),
        }
    }
}

pub(super) struct GridSearchConfig {
    pub(super) thresholds: Vec<f32>,
    pub(super) shifts: Vec<f32>,
    pub(super) min_pos_cois: Vec<usize>,
    pub(super) click_probability: f64,
    pub(super) ndocuments: usize,
    pub(super) iterations: usize,
    pub(super) nranks: Vec<usize>,
    pub(super) is_semi_interesting: bool,
}

impl Default for GridSearchConfig {
    fn default() -> Self {
        Self {
            thresholds: vec![0.67, 0.7, 0.75, 0.8, 0.85, 0.9],
            shifts: vec![0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4],
            min_pos_cois: vec![1],
            click_probability: 0.2,
            ndocuments: 100,
            iterations: 10,
            nranks: vec![3, 5],
            is_semi_interesting: false,
        }
    }
}

pub(super) fn create_grid_search_configs(
    grid_search_config: &GridSearchConfig,
) -> Vec<StateConfig> {
    let mut configs = Vec::with_capacity(
        grid_search_config.thresholds.len()
            * grid_search_config.shifts.len()
            * grid_search_config.min_pos_cois.len(),
    );

    let start_time = Utc::now();

    for t in &grid_search_config.thresholds {
        for s in &grid_search_config.shifts {
            for m in &grid_search_config.min_pos_cois {
                configs.push(StateConfig {
                    coi: {
                        CoiConfig::default()
                            .with_shift_factor(*s)
                            .unwrap()
                            .with_threshold(*t)
                            .unwrap()
                            .with_min_positive_cois(*m)
                            .unwrap()
                    },
                    personalization: PersonalizationConfig::default(),
                    time: start_time,
                });
            }
        }
    }
    configs
}

/// The config of hyperparameters for the persona based benchmark.
#[derive(Debug, Deserialize)]
pub(super) struct PersonaBasedConfig {
    pub(super) click_probability: f64,
    pub(super) ndocuments: usize,
    pub(super) iterations: usize,
    pub(super) amount_of_doc_used_to_prepare: usize,
    pub(super) nranks: Vec<usize>,
    pub(super) ndocuments_hot_news: usize,
    pub(super) is_semi_interesting: bool,
}

impl Default for PersonaBasedConfig {
    fn default() -> Self {
        Self {
            click_probability: 0.2,
            ndocuments: 100,
            iterations: 10,
            amount_of_doc_used_to_prepare: 1,
            nranks: vec![3, 5],
            ndocuments_hot_news: 15,
            is_semi_interesting: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct SaturationConfig {
    pub(super) click_probability: f64,
    pub(super) ndocuments: usize,
    pub(super) iterations: usize,
}

impl Default for SaturationConfig {
    fn default() -> Self {
        Self {
            click_probability: 0.2,
            ndocuments: 30,
            iterations: 10,
        }
    }
}
