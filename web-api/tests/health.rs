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

use reqwest::{Client, StatusCode};
use xayn_integration_tests::{send_assert, test_app, UNCHANGED_CONFIG};
use xayn_web_api::Ingestion;

#[test]
fn test_health() {
    test_app::<Ingestion, _>(UNCHANGED_CONFIG, |_client, url, _| async move {
        // make sure not to use any presets from `test_app`, like e.g. the
        // X-Xayn-Tenant-Id header.
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        send_assert(
            &client,
            client.get(url.join("/health")?).build()?,
            StatusCode::OK,
            false,
        )
        .await;
        Ok(())
    });
}
