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

//! A small binary utility to allow using the existing providers from a CLI.
//!
//! This can be useful for various debugging testing use cases.
//!
//! This calling this tool will fetch articles and then dump the `Article`
//! **as returned by the provider** as json files.
//!
//! If you want raw input returned from the remote endpoint and feed into the
//! provider use `curl` instead.
//!
//! [Here is the documentation for the remote newscatcher API.](https://docs.newscatcherapi.com/api-docs/endpoints/search-news)
//!
//! [And here for the remote gnews API.](https://gnews.io/docs/v4#search-endpoint)

use std::{env, fs, path::PathBuf, thread::sleep, time::Duration};

use anyhow::{bail, Context, Result};
use tokio::runtime::Runtime;
use url::Url;
use xayn_discovery_engine_providers::{
    gnews,
    newscatcher,
    Article,
    CommonQueryParts,
    Filter,
    HeadlinesProvider,
    HeadlinesQuery,
    NewsProvider,
    NewsQuery,
};

/// Some hard coded well known urls/paths which we likely currently use.
mod well_known {
    pub(super) const BASE_URL: &str = "https://api-gw.xaynet.dev";
    pub(super) const GNEWS_HEADLINES_PATH: &str = "/gnews/v2/latest-headlines";
    pub(super) const GNEWS_SEARCH_NEWS_PATH: &str = "/gnews/v2/search-news";
    pub(super) const NEWSCATCHER_HEADLINES_PATH: &str = "/newscatcher/v1/latest-headlines";
    pub(super) const NEWSCATCHER_SEARCH_NEWS_PATH: &str = "/newscatcher/v1/search-news";
}

fn resolve_default_url_paths(provider: &str, method: &str) -> String {
    use well_known::*;

    match (provider, method) {
        ("gnews", "search-news") => format!("{}{}", BASE_URL, GNEWS_SEARCH_NEWS_PATH),
        ("gnews", "latest-headlines") => format!("{}{}", BASE_URL, GNEWS_HEADLINES_PATH),
        ("newscatcher", "search-news") => format!("{}{}", BASE_URL, NEWSCATCHER_SEARCH_NEWS_PATH),
        ("newscatcher", "latest-headlines") => {
            format!("{}{}", BASE_URL, NEWSCATCHER_HEADLINES_PATH)
        }
        _ => panic!("Unsupported provider or method"),
    }
}

/// Usage `cargo run --bin test_provider -- gnews|newscatcher search-news|latest-headlines [<full_url_to_endpoint>]`.
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let provider = args
        .get(1)
        .context("first argument must be the provider name")?;
    let method = args.get(2).context("second argument must be the method")?;
    let url = args.get(3).map_or_else(
        || resolve_default_url_paths(provider, method),
        |s| s.to_owned(),
    );
    let url = Url::parse(&url).unwrap();
    let auth_token = std::env::var("PROVIDER_DEV_BEARER_AUTH_TOKEN").context(
        "Please provide the PROVIDER_DEV_BEARER_AUTH_TOKEN environment variable for the dev environment. \
                  The token can be found in 1Password",
    )?;

    let download_dir = setup_download_dir(provider, method)?;

    let args = DownloadArgs {
        download_dir,
        max_nr_batches: 10,
        page_size: 10,
    };

    match method.as_str() {
        "search-news" => download(args, create_news_downloader(provider, url, auth_token)?)?,
        "latest-headlines" => download(
            args,
            create_headlines_downloader(provider, url, auth_token)?,
        )?,
        _ => bail!("Unsupported method"),
    }
    Ok(())
}

fn create_news_downloader(
    provider: &str,
    url: Url,
    auth_token: String,
) -> Result<impl FnMut(usize, usize) -> Vec<Article>> {
    let rt = Runtime::new().unwrap();
    let market = ("US", "en").into();

    let provider: Box<dyn NewsProvider> = match provider {
        "newscatcher" => Box::new(newscatcher::NewsProviderImpl::new(url, auth_token)),
        "gnews" => Box::new(gnews::NewsProviderImpl::new(url, auth_token)),
        _ => bail!("unknown provider"),
    };
    let filter = Filter::default().add_keyword("Germany");

    Ok(move |page, page_size| {
        let query = NewsQuery {
            common: CommonQueryParts {
                page_size,
                page,
                excluded_sources: &[],
            },
            market: &market,
            filter: &filter,
            from: None,
        };
        rt.block_on(provider.query_news(&query)).unwrap()
    })
}

fn create_headlines_downloader(
    provider: &str,
    url: Url,
    auth_token: String,
) -> Result<impl FnMut(usize, usize) -> Vec<Article>> {
    let rt = Runtime::new().unwrap();
    let market = ("US", "en").into();

    let provider: Box<dyn HeadlinesProvider> = match provider {
        "newscatcher" => Box::new(newscatcher::HeadlinesProviderImpl::new(url, auth_token)),
        "gnews" => Box::new(gnews::HeadlinesProviderImpl::new(url, auth_token)),
        _ => bail!("unknown provider"),
    };

    Ok(move |page, page_size| {
        let query = HeadlinesQuery {
            common: CommonQueryParts {
                page_size,
                page,
                excluded_sources: &[],
            },
            market: &market,
            topic: None,
            when: None,
        };
        rt.block_on(provider.query_headlines(&query)).unwrap()
    })
}

fn setup_download_dir(provider: &str, method: &str) -> Result<PathBuf> {
    let mut base_path = PathBuf::from("./provider_downloads");
    base_path.push(provider);
    base_path.push(method);

    fs::create_dir_all(&base_path)
        .with_context(|| format!("Failed to create: {}", base_path.display()))?;

    Ok(base_path)
}

struct DownloadArgs {
    download_dir: PathBuf,
    max_nr_batches: usize,
    page_size: usize,
}

fn download(
    args: DownloadArgs,
    mut download_batch: impl FnMut(usize, usize) -> Vec<Article>,
) -> Result<()> {
    for page in 1..=args.max_nr_batches {
        println!("start downloading page {}", page);

        let articles = download_batch(page, args.page_size);

        let out_file = args.download_dir.join(format!("page_{:03}.json", page));
        let json = serde_json::to_string_pretty(&articles)
            .context("json serialization of articles failed")?;
        fs::write(&out_file, json)
            .with_context(|| format!("Writing json file failed: {}", out_file.display()))?;

        if articles.len() < args.page_size {
            break;
        }

        println!("page {} written to {}", page, out_file.display());
        // Needed for some providers (gnews) which do not allow to many queries to fast
        // (at least not with the dev keys we have).
        sleep(Duration::from_millis(1000));
    }
    Ok(())
}
