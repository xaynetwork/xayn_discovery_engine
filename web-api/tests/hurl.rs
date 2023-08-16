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

use std::collections::HashMap;

use hurl::{
    runner,
    runner::{RunnerOptionsBuilder, Value},
    util::logger::{LoggerOptionsBuilder, Verbosity},
};

use walkdir::WalkDir;
use xayn_integration_tests::{test_app, UNCHANGED_CONFIG};
use xayn_web_api::Ingestion;

#[test]
fn run_hurl_tests() {
    let runner_opts = RunnerOptionsBuilder::new().follow_location(true).build();

    // Set variables
    let mut variables = HashMap::new();
    // variables.insert("host".to_string(), Value::String(url));
    variables.insert(
        "bo_token".to_string(),
        Value::String("no_bo_token_locally".to_string()),
    );
    variables.insert(
        "fo_token".to_string(),
        Value::String("no_fo_token_locally".to_string()),
    );

    for entry in WalkDir::new("tests/hurl.d")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let content = std::fs::read_to_string(entry.path()).unwrap();

        let runner_opts = runner_opts.clone();
        let mut variables = variables.clone();
        let logger_opts = LoggerOptionsBuilder::new()
            .verbosity(Some(Verbosity::Verbose))
            .build();

        test_app::<Ingestion, _>(UNCHANGED_CONFIG, |_, url, _| async move {
            variables.insert("url".to_string(), Value::String(url.to_string()));

            let result = runner::run(
                &content,
                &runner_opts.clone(),
                &variables.clone(),
                &logger_opts,
            );

            assert!(result.unwrap().success);

            Ok(())
        })
    }
}
