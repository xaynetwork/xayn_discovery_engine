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

extern crate core;

use anyhow::{Context, Result};
use std::{env, time::Duration};
use tokio::time::sleep;
use xayn_discovery_engine_providers::{
    gnews_client,
    newscatcher_client,
    CommonQueryParts,
    Filter,
    GnewsHeadlinesQuery,
    GnewsNewsQuery,
    HeadlinesQuery,
    Market,
};

#[tokio::main]
async fn main() -> Result<()> {
    let url = "https://api-gw.xaynet.dev".to_string();
    let token = std::env::var("NEWSCATCHER_DEV_BEARER_AUTH_TOKEN").context(
        "Please provide the NEWSCATCHER_DEV_BEARER_AUTH_TOKEN environment variable for the dev environment. \
                  The token can be found in 1Password",
    )?;

    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.to_owned()).unwrap_or_default();

    match mode.as_str() {
        "newscatcher" => query_newscatcher(&url, &token).await,
        "gnews_search" => query_gnews(&url, &token, false).await,
        "gnews_headlines" => query_gnews(&url, &token, true).await,
        _ => panic!("Unknown mode {}", mode),
    }
}

async fn query_gnews(url: &str, token: &str, headlines: bool) -> Result<()> {
    tokio::fs::create_dir("./gnews_download")
        .await
        .context("Failed to create download directory. Does it already exist?")?;

    let client = gnews_client::Client::new(token, url);
    let market = Market {
        lang_code: "en".to_string(),
        country_code: "US".to_string(),
    };

    let total_pages = 10;
    let mut page = 1;
    while page <= total_pages {
        println!("Fetching page {} of {}", page, total_pages);

        let content = if headlines {
            let params = GnewsHeadlinesQuery {
                market: Some(&market),
                page_size: 10,
                page,
                excluded_sources: &[],
                filter: None,
            };
            let response = client.query_headlines(&params).await.unwrap();
            serde_json::to_string_pretty(&response)?
        } else {
            let filter = Filter::default().add_keyword("clouds");
            let params = GnewsNewsQuery {
                market: Some(&market),
                page_size: 10,
                page,
                excluded_sources: &[],
                filter: &filter,
            };

            let response = client.query_articles(&params).await.unwrap();
            serde_json::to_string_pretty(&response)?
        };

        tokio::fs::write(format!("./gnews_download/page_{:03}.json", page), content).await?;

        page += 1;

        // Wait a little, because Gnews has very strict requests/second limitations
        sleep(Duration::from_millis(1000)).await;
    }

    Ok(())
}

async fn query_newscatcher(url: &str, token: &str) -> Result<()> {
    tokio::fs::create_dir("./headlines_download")
        .await
        .context("Failed to create download directory. Does it already exist?")?;

    let client = newscatcher_client::Client::new(token, url);
    let market = Market {
        lang_code: "en".to_string(),
        country_code: "US".to_string(),
    };

    // This is updated every iteration, based on the response from Newscatcher. So in reality,
    // we'll be fetching more than one page.
    let mut total_pages = 1;
    let mut page = 1;
    while page <= total_pages {
        println!("Fetching page {} of {}", page, total_pages);
        let params = HeadlinesQuery {
            common: CommonQueryParts {
                market: Some(&market),
                page_size: 100,
                page,
                excluded_sources: &[],
            },
            trusted_sources: &[],
            topic: None,
            when: None,
        };
        let raw_response = client.query(&params).await.unwrap();
        total_pages = raw_response.total_pages;

        let content = serde_json::to_string_pretty(&raw_response.articles)?;
        tokio::fs::write(
            format!("./headlines_download/page_{:03}.json", page),
            content,
        )
        .await?;

        page += 1;
    }

    Ok(())
}
