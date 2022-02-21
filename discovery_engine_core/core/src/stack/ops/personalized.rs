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

use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use tokio::sync::RwLock;
use uuid::Uuid;
use xayn_ai::ranker::KeyPhrase;

use xayn_discovery_engine_providers::{Article, Market, Topic};

use crate::{
    document::Document,
    engine::{EndpointConfig, GenericError},
    stack::Id,
};

use super::Ops;

/// Stack operations customized for personalized news items.
// NOTE mock implementation for now
#[derive(Default)]
pub(crate) struct PersonalizedNews {
    token: String,
    url: String,
    markets: Option<Arc<RwLock<Vec<Market>>>>,
}

#[async_trait]
impl Ops for PersonalizedNews {
    fn id(&self) -> Id {
        Id(Uuid::parse_str("311dc7eb-5fc7-4aa4-8232-e119f7e80e76").unwrap(/* valid uuid */))
    }

    fn configure(&mut self, config: &EndpointConfig) {
        self.token.clone_from(&config.api_key);
        self.url.clone_from(&config.api_base_url);
        self.markets.replace(Arc::clone(&config.markets));
    }

    #[allow(clippy::cast_precision_loss)]
    #[allow(clippy::cast_possible_truncation)]
    async fn new_items(&self, _key_phrases: &[KeyPhrase]) -> Result<Vec<Article>, GenericError> {
        let n = 10;
        let articles = (0..n).fold(Vec::with_capacity(n), |mut articles, i| {
            articles.push(
            Article {
                id: i.to_string(),
                title: format!("P Document Title {}", i),
                score: if i % 2 == 0 {Some(i as f32) } else {None},
                rank: i,
                source_domain: "xayn.com".to_string(),
                excerpt: format!("Content of the news {}", i),
                link: "https://xayn.com/".into(),
                media: "https://uploads-ssl.webflow.com/5ea197660b956f76d26f0026/614349038d7d72d1576ae3f4_plant.svg".into(),
                topic: Topic::Unrecognized,
                country: "DE".to_string(),
                language: "de".to_string(),
                published_date: NaiveDate::from_ymd(2022, 2, (i + 1) as u32).and_hms(9, 10, 11),
            });

            articles
        });

        Ok(articles)
    }

    fn filter_articles(
        &self,
        _current: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        Ok(articles)
    }

    fn merge(&self, current: &[Document], new: &[Document]) -> Result<Vec<Document>, GenericError> {
        let mut res: Vec<_> = current.into();
        res.extend_from_slice(new);
        Ok(res)
    }
}
