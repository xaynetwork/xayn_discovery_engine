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
    #[serde(rename(deserialize = "_id"))]
    pub id: String,

    /// The title of the article.
    pub title: String,

    /// How well the article is matching your search criteria.
    #[serde(rename(deserialize = "_score"))]
    pub score: Option<f32>,

    /// The page rank of the source website.
    pub rank: usize,

    /// The URL of the article's source.
    pub clean_url: String,

    /// Short summary of the article provided by the publisher.
    pub excerpt: String,

    /// Full URL where the article was originally published.
    pub link: String,

    /// A link to a thumbnail image of the article.
    pub media: String,

    /// The main topic of the news publisher.
    /// Important: This parameter is not deducted on a per-article level:
    /// it is deducted on the per-publisher level.
    pub topic: Topic,

    /// The country of the publisher.
    pub country: String,

    /// The language of the article.
    pub language: String,

    /// While Newscatcher claims to have some sort of timezone support in their
    /// [API][<https://docs.newscatcherapi.com/api-docs/endpoints/search-news>] (via the
    /// `published_date_precision` attribute), in practice they do not seem to be supplying any
    /// sort of timezone information. As a result, we provide NaiveDateTime for now.
    #[serde(deserialize_with = "naive_date_time_from_str")]
    pub published_date: NaiveDateTime,
}

fn naive_date_time_from_str<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(de::Error::custom)
}

#[derive(Deserialize, Debug)]
pub(crate) struct Response {
    pub(crate) status: String,
    pub(crate) articles: Vec<Article>,
}
