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

//! Executes the user-based MIND benchmark.
use std::{collections::HashMap, fs::File};

use csv::{DeserializeRecordsIntoIter, Reader, ReaderBuilder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Impression {
    id: String,
    user_id: String,
    time: String,
    clicks: String,
    news: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Article {
    id: String,
    category: String,
    subcategory: String,
    title: String,
    snippet: String,
    url: String,
}

/// Reads and deserializes news data from a tsv file path and returns an iterator
fn read_articles(path: &str) -> Result<DeserializeRecordsIntoIter<File, Article>, anyhow::Error> {
    let iter = read_from_tsv(path)?.into_deserialize();

    Ok(iter)
}

/// Reads and deserializes impressions data from a tsv file path and returns an iterator
fn read_impressions(
    path: &str,
) -> Result<DeserializeRecordsIntoIter<File, Impression>, anyhow::Error> {
    let iter = read_from_tsv(path)?.into_deserialize();

    Ok(iter)
}

/// Reads data from a tsv file path into a reader
fn read_from_tsv(path: &str) -> Result<Reader<File>, anyhow::Error> {
    ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(path)
        .map_err(Into::into)
}

/// Runs the user-based mind benchmark
fn run_benchmark() -> Result<(), anyhow::Error> {
    let mut articles_map = HashMap::new();
    let articles_iter = read_articles("news.tsv")?;

    for article in articles_iter {
        let article: Article = article?;
        articles_map.entry(article.id.clone()).or_insert(article);
    }

    let impressions_iter = read_impressions("behaviors.tsv")?;

    for impression in impressions_iter {
        let impression: Impression = impression?;
        let clicks = impression.clicks.split(' ').collect::<Vec<&str>>();
        match articles_map.get(clicks[0]) {
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
