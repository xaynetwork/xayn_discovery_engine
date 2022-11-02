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

use std::time::Duration;

use chrono::{DateTime, Utc};
use url::Url;
use xayn_discovery_engine_ai::Embedding;
use xayn_discovery_engine_providers::Market;

use crate::{
    document::{self, UserReaction},
    stack,
};

#[derive(Debug)]
pub(crate) struct NewDocument {
    pub(crate) id: document::Id,
    pub(crate) news_resource: NewsResource,
    pub(crate) newscatcher_data: NewscatcherData,
    pub(crate) embedding: Embedding,
}

impl From<document::Document> for NewDocument {
    fn from(doc: document::Document) -> Self {
        let (news_resource, newscatcher_data) = doc.resource.into();
        Self {
            id: doc.id,
            news_resource,
            newscatcher_data,
            embedding: doc.bert_embedding,
        }
    }
}

impl From<document::NewsResource> for (NewsResource, NewscatcherData) {
    fn from(resource: document::NewsResource) -> Self {
        let news_resource = NewsResource {
            title: resource.title,
            snippet: resource.snippet,
            topic: resource.topic,
            url: resource.url,
            image: resource.image,
            date_published: resource.date_published,
            source: resource.source_domain,
            market: Market::new(resource.language, resource.country),
        };
        let newscatcher_data = NewscatcherData {
            domain_rank: resource.rank,
            score: resource.score,
        };
        (news_resource, newscatcher_data)
    }
}

impl From<(NewsResource, NewscatcherData)> for document::NewsResource {
    fn from((news_resource, newscatcher_data): (NewsResource, NewscatcherData)) -> Self {
        Self {
            title: news_resource.title,
            snippet: news_resource.snippet,
            url: news_resource.url,
            source_domain: news_resource.source,
            date_published: news_resource.date_published,
            image: news_resource.image,
            rank: newscatcher_data.domain_rank,
            score: newscatcher_data.score,
            country: news_resource.market.country_code,
            language: news_resource.market.lang_code,
            topic: news_resource.topic,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ApiDocumentView {
    pub(crate) document_id: document::Id,
    pub(crate) news_resource: NewsResource,
    pub(crate) newscatcher_data: NewscatcherData,
    pub(crate) user_reaction: Option<UserReaction>,
    pub(crate) embedding: Embedding,
    pub(crate) stack_id: Option<stack::Id>,
}

impl ApiDocumentView {
    /// Gets the snippet or falls back to the title if the snippet is empty.
    pub(crate) fn snippet_or_title(&self) -> &str {
        if self.news_resource.snippet.is_empty() {
            &self.news_resource.title
        } else {
            &self.news_resource.snippet
        }
    }
}

impl From<ApiDocumentView> for document::Document {
    fn from(view: ApiDocumentView) -> Self {
        document::Document {
            id: view.document_id,
            stack_id: view.stack_id.unwrap_or_default(),
            bert_embedding: view.embedding,
            reaction: view.user_reaction,
            resource: (view.news_resource, view.newscatcher_data).into(),
        }
    }
}

/// Represents news that is delivered by an external content API.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NewsResource {
    /// Title of the resource.
    pub(crate) title: String,

    /// Snippet of the resource.
    pub(crate) snippet: String,

    /// Main topic of the publisher.
    pub(crate) topic: String,

    /// Url to reach the resource.
    pub(crate) url: Url,

    /// Image attached to the news.
    pub(crate) image: Option<Url>,

    /// Publishing date.
    pub(crate) date_published: DateTime<Utc>,

    /// The domain of the article's source, e.g. `example.com`. Not a valid URL.
    pub(crate) source: String,

    /// The market of news.
    pub(crate) market: Market,
}

#[derive(Debug, PartialEq)]
pub(crate) struct NewscatcherData {
    pub(crate) domain_rank: u64,
    pub(crate) score: Option<f32>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct TimeSpentDocumentView {
    pub(crate) bert_embedding: Embedding,
    pub(crate) last_reaction: Option<UserReaction>,
    pub(crate) aggregated_view_time: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Search {
    pub search_by: SearchBy,
    pub search_term: String,
    pub paging: Paging,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, num_derive::FromPrimitive)]
#[repr(u8)]
pub enum SearchBy {
    Query = 0,
    Topic = 1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paging {
    pub size: u32,
    pub next_page: u32,
}
