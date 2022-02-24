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
use derive_more::Display;
use serde::{de, Deserialize, Deserializer, Serialize};

/// Topic of the publisher.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs)]
pub enum Topic {
    News,
    Sport,
    Tech,
    World,
    Finance,
    Politics,
    Business,
    Economics,
    Entertainment,
    Beauty,
    Travel,
    Music,
    Food,
    Science,
    Gaming,
    Energy,
    #[serde(other)]
    Unrecognized,
}

/// A news article
#[derive(Clone, Deserialize, Debug)]
pub struct Article {
    /// Newscatcher API's unique identifier for each news article.
    #[serde(
        rename(deserialize = "_id"),
        deserialize_with = "deserialize_null_default"
    )]
    pub id: String,

    /// The title of the article.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub title: String,

    /// How well the article is matching your search criteria.
    #[serde(
        rename(deserialize = "_score"),
        deserialize_with = "deserialize_null_default"
    )]
    pub score: Option<f32>,

    /// The page rank of the source website.
    #[serde(deserialize_with = "deserialize_rank")]
    pub rank: usize,

    /// The domain of the article's source, e.g. `xayn.com`. Not a valid URL.
    #[serde(
        rename(deserialize = "clean_url"),
        deserialize_with = "deserialize_null_default"
    )]
    pub source_domain: String,

    /// Short summary of the article provided by the publisher.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub excerpt: String,

    /// Full URL where the article was originally published.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub link: String,

    /// A link to a thumbnail image of the article.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub media: String,

    /// The main topic of the news publisher.
    /// Important: This parameter is not deducted on a per-article level:
    /// it is deducted on the per-publisher level.
    #[serde(deserialize_with = "deserialize_topic")]
    pub topic: Topic,

    /// The country of the publisher.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub country: String,

    /// The language of the article.
    #[serde(deserialize_with = "deserialize_null_default")]
    pub language: String,

    /// While Newscatcher claims to have some sort of timezone support in their
    /// [API][<https://docs.newscatcherapi.com/api-docs/endpoints/search-news>] (via the
    /// `published_date_precision` attribute), in practice they do not seem to be supplying any
    /// sort of timezone information. As a result, we provide NaiveDateTime for now.
    #[serde(deserialize_with = "deserialize_naive_date_time_from_str")]
    pub published_date: NaiveDateTime,
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
fn deserialize_rank<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(usize::MAX))
}

/// Null-value tolerant deserialization of topic
fn deserialize_topic<'de, D>(deserializer: D) -> Result<Topic, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or(Topic::News))
}

#[derive(Deserialize, Debug)]
pub(crate) struct Response {
    pub(crate) status: String,
    pub(crate) articles: Vec<Article>,
}
