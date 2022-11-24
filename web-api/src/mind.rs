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
#![allow(dead_code)]

//! Executes the user-based MIND benchmark.
use std::{collections::HashMap, fs::File, path::Path};

use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Impression {
    id: String,
    user_id: String,
    time: String,
    clicks: String,
    news: String,
}

#[derive(Debug, Deserialize)]
struct Article {
    id: String,
    category: String,
    subcategory: String,
    title: String,
    snippet: String,
    url: String,
}

fn read<T>(path: &str) -> Result<DeserializeRecordsIntoIter<File, T>, anyhow::Error>
where
    for<'de> T: Deserialize<'de>,
{
    Ok(read_from_tsv(path)?.into_deserialize())
}

fn read_from_tsv<P>(path: P) -> Result<Reader<File>, anyhow::Error>
where
    P: AsRef<Path>,
{
    ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(path)
        .map_err(Into::into)
}

/// Runs the user-based mind benchmark
fn run_benchmark() -> Result<(), anyhow::Error> {
    let articles = read("news.tsv")?
        .map(|result| result.map(|article: Article| (article.id.clone(), article)))
        .collect::<Result<HashMap<_, _>, _>>()?;

    let impressions_iter = read("behaviors.tsv")?;

    // Loop over all impressions, prepare reranker with news in click history
    // and rerank the news in an impression
    for impression in impressions_iter {
        let impression: Impression = impression?;
        let clicks = impression.clicks.split(' ').collect::<Vec<&str>>();
        match articles.get(clicks[0]) {
            Some(imp) => println!("{:?}", imp),
            None => println!("Article id {} not found.", clicks[0]),
        }

    }

    Ok(())
}

fn main() {
    if let Err(e) = run_benchmark() {
        eprintln!("{}", e);
    }
}
