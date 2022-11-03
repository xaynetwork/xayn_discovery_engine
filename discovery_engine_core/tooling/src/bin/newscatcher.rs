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

//! Calls the newscatcher api.

#![forbid(unsafe_code, unsafe_op_in_unsafe_fn)]
#![deny(
    clippy::future_not_send,
    clippy::pedantic,
    noop_method_call,
    rust_2018_idioms,
    unused_qualifications
)]
#![warn(unreachable_pub, rustdoc::missing_crate_level_docs)]
#![allow(
    clippy::items_after_statements,
    clippy::missing_errors_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]

use anyhow::{Context, Result};
use xayn_discovery_engine_providers::{
    Config,
    HeadlinesQuery,
    Market,
    NewscatcherHeadlinesProvider,
    RankLimit,
    RestEndpoint,
};

#[tokio::main]
async fn main() -> Result<()> {
    let base_url = "https://api-gw.xaynet.dev";
    let token = std::env::var("NEWSCATCHER_DEV_BEARER_AUTH_TOKEN").context(
        "Please provide the NEWSCATCHER_DEV_BEARER_AUTH_TOKEN environment variable for the dev environment. \
                  The token can be found in 1Password",
    )?;
    let config = Config::headlines(base_url, None, token)?
        .with_timeout(3500)
        .with_retry(3);
    let provider = NewscatcherHeadlinesProvider::from_endpoint(RestEndpoint::new(config));

    tokio::fs::create_dir("./headlines_download")
        .await
        .context("Failed to create download directory. Does it already exist?")?;

    // This is updated every iteration, based on the response from Newscatcher. So in reality,
    // we'll be fetching more than one page.
    let mut page = 1;
    let market = Market::new("en", "US");
    loop {
        println!("Fetching page {}", page);
        let params = HeadlinesQuery {
            market: &market,
            page_size: 100,
            page,
            rank_limit: RankLimit::LimitedByMarket,
            excluded_sources: &[],
            trusted_sources: &[],
            topic: None,
            max_age_days: None,
        };
        let articles = provider.query_headlines(&params).await.unwrap();
        if articles.is_empty() {
            break;
        }

        let content = serde_json::to_string_pretty(&articles)?;
        tokio::fs::write(
            format!("./headlines_download/page_{:03}.json", page),
            content,
        )
        .await?;

        page += 1;
    }

    Ok(())
}
