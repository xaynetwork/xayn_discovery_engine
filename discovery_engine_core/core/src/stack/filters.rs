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

use crate::{document::Document, engine::GenericError};
use std::collections::HashSet;
use url::Url;
use xayn_discovery_engine_providers::Article;

pub(crate) trait ArticleFilter {
    fn apply(current: &[Document], articles: Vec<Article>) -> Result<Vec<Article>, GenericError>;
}

struct DuplicateFilter;

impl ArticleFilter for DuplicateFilter {
    fn apply(
        current: &[Document],
        mut articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        let mut urls = HashSet::new();
        for doc in current {
            urls.insert(doc.resource.url.as_str());
        }

        articles.retain(|article| !urls.contains(&article.link.as_str()));
        Ok(articles)
    }
}

struct MalformedImageUrlFilter;

impl ArticleFilter for MalformedImageUrlFilter {
    fn apply(
        _current: &[Document],
        mut articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        articles.retain(|article| Url::parse(&article.media).is_ok());
        Ok(articles)
    }
}

pub(crate) struct CommonFilter;
impl ArticleFilter for CommonFilter {
    fn apply(current: &[Document], articles: Vec<Article>) -> Result<Vec<Article>, GenericError> {
        DuplicateFilter::apply(current, articles)
            .and_then(|articles| MalformedImageUrlFilter::apply(current, articles))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        document::{document_from_article, Document},
        stack::filters::CommonFilter,
    };
    use xayn_discovery_engine_providers::Article;

    #[test]
    fn test_duplicate_filter() {
        let valid_articles: Vec<Article> =
            serde_json::from_str(include_str!("../../test-fixtures/articles-valid.json")).unwrap();

        let documents = valid_articles.as_slice()[0..2]
            .iter()
            .map(|article| {
                let doc = Document::default();
                document_from_article(article.clone(), doc.stack_id, doc.smbert_embedding).unwrap()
            })
            .collect::<Vec<_>>();

        let result = CommonFilter::apply(documents.as_slice(), valid_articles).unwrap();
        let titles = result.iter().map(|a| &a.title).collect::<Vec<_>>();

        assert_eq!(titles, [
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
        ]);
    }

    #[test]
    fn test_malformed_media_filter() {
        let documents: Vec<Document> = vec![];
        let valid_articles: Vec<Article> =
            serde_json::from_str(include_str!("../../test-fixtures/articles-valid.json")).unwrap();
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../test-fixtures/articles-some-malformed-media-urls.json"
        ))
        .unwrap();

        let input: Vec<Article> = valid_articles
            .iter()
            .cloned()
            .chain(malformed_articles.iter().cloned())
            .collect();

        let result = CommonFilter::apply(documents.as_slice(), input).unwrap();
        let titles = result.iter().map(|a| &a.title).collect::<Vec<_>>();

        assert_eq!(titles.as_slice(), [
            "Olympic champion Lundby laments ski jumping's weight issues",
            "Jerusalem blanketed in white after rare snowfall",
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
        ]);
    }
}
