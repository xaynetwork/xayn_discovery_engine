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

use std::{
    num::{NonZeroU16, NonZeroU64},
    time::Duration,
};

use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use tokio::{time::sleep, try_join};

use crate::{
    config,
    logging::{self, init_tracing},
    storage,
};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    logging: logging::Config,
    storage: storage::Config,
    n: NonZeroU16,
    t: NonZeroU64,
}

pub async fn start() -> Result<(), anyhow::Error> {
    let config = config::load::<Config>(["XAYN_MIGRATION"]);

    init_tracing(&config.logging);
    let storage = config.storage.setup().await?;

    let n = config.n;
    let t = Duration::from_secs(config.t.get());
    loop {
        match try_join!(storage.migrate(n), sleep(t).map(Ok)) {
            Ok((all_migrated, ())) => {
                if all_migrated {
                    break;
                }
            }
            Err(error) => {
                storage.close().await;
                return Err(error.into());
            }
        }
    }

    storage.close().await;

    Ok(())
}