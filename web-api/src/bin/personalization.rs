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

use tracing::instrument;
use xayn_web_api::{config, start, Application, Personalization};

#[tokio::main]
#[instrument(err)]
async fn main() -> Result<(), anyhow::Error> {
    let config = config::load(&[Personalization::NAME, "XAYN_WEB_API"]);
    start::<Personalization>(config)
        .await?
        .wait_for_termination()
        .await
}
