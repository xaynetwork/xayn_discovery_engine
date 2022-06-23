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

use chrono::NaiveDateTime;
use serde::{de, Deserialize, Deserializer, Serialize};

/// A news article
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Article {
    /// The title of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub title: String,

    /// How well the article is matching your search criteria.
    #[serde(
        default,
        rename(deserialize = "_score"),
        deserialize_with = "deserialize_null_default"
    )]
    pub score: Option<f32>,

    /// The page rank of the source website.
    #[serde(default, deserialize_with = "deserialize_rank")]
    pub rank: u64,

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    #[serde(
        default,
        rename(deserialize = "clean_url"),
        deserialize_with = "deserialize_null_default"
    )]
    pub source_domain: String,

    /// Short summary of the article provided by the publisher.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub excerpt: String,

    /// Full URL where the article was originally published.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub link: String,

    /// A link to a thumbnail image of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub media: String,

    /// The main topic of the news publisher.
    /// Important: This parameter is not deducted on a per-article level:
    /// it is deducted on the per-publisher level.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub topic: String,

    /// The country of the publisher.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub country: String,

    /// The language of the article.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub language: String,

    /// While Newscatcher claims to have some sort of timezone support in their
    /// [API][<https://docs.newscatcherapi.com/api-docs/endpoints/search-news>] (via the
    /// `published_date_precision` attribute), in practice they do not seem to be supplying any
    /// sort of timezone information. As a result, we provide NaiveDateTime for now.
    #[serde(
        default = "default_published_date",
        deserialize_with = "deserialize_naive_date_time_from_str"
    )]
    pub published_date: NaiveDateTime,
}

fn default_published_date() -> NaiveDateTime {
    chrono::naive::MIN_DATETIME
}

// Taken from https://github.com/serde-rs/serde/issues/1098#issuecomment-760711617
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Null-value tolerant deserialization of `NaiveDateTime`
fn deserialize_naive_date_time_from_str<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    opt.map_or_else(
        || Ok(NaiveDateTime::from_timestamp(0, 0)),
        |s| NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(de::Error::custom),
    )
}

/// Null-value tolerant deserialization of rank
fn deserialize_rank<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(u64::MAX))
}

/// Query response from the Newscatcher API
#[derive(Deserialize, Debug)]
pub struct Response {
    /// Status message
    pub status: String,
    /// Main response content
    #[serde(default)]
    pub articles: Vec<Article>,
    /// Total pages of content available
    pub total_pages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    impl Default for Article {
        fn default() -> Self {
            Article {
                title: "title".to_string(),
                score: Some(0.75),
                rank: 10,
                source_domain: "example.com".to_string(),
                excerpt: "summary of the article".to_string(),
                link: "https://example.com/news/".to_string(),
                media: "https://example.com/news/image/".to_string(),
                topic: "news".to_string(),
                country: "EN".to_string(),
                language: "en".to_string(),
                published_date: NaiveDate::from_ymd(2022, 1, 1).and_hms(9, 0, 0),
            }
        }
    }

    #[test]
    // In order to make sure that our API clients don't throw errors if some articles
    // are malformed (missing fields, null fields) we are very liberal in what we
    // accept as articles, and will filter out malformed ones further down the processing
    // chain.
    fn test_deserialize_article_where_all_fields_should_fall_back_to_default() {
        let _article: Article = serde_json::from_str("{}").unwrap();
    }
}
