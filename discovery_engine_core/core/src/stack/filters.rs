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

use std::{borrow::Cow, collections::HashSet};

use url::Url;

use crate::{
    document::{Document, HistoricDocument},
    engine::GenericError,
};
use xayn_discovery_engine_providers::Article;

pub(crate) trait ArticleFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError>;
}

struct DuplicateFilter;

impl ArticleFilter for DuplicateFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        mut articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        let mut urls = history
            .iter()
            .map(|doc| Cow::Borrowed(doc.url.as_str()))
            .chain(
                stack
                    .iter()
                    .map(|doc| Cow::Borrowed(doc.resource.url.as_str())),
            )
            .collect::<HashSet<_>>();

        let mut titles = history
            .iter()
            .map(|doc| Cow::Borrowed(&doc.title))
            .chain(stack.iter().map(|doc| Cow::Borrowed(&doc.resource.title)))
            .collect::<HashSet<_>>();

        articles.retain(|article| {
            let do_retain = !(urls.contains(&Cow::Borrowed(article.link.as_str()))
                || titles.contains(&Cow::Borrowed(&article.title)));
            urls.insert(Cow::Owned(article.link.to_string()));
            titles.insert(Cow::Owned(article.title.clone()));
            do_retain
        });

        Ok(articles)
    }
}

struct MalformedFilter;

impl MalformedFilter {
    fn is_valid(article: &Article) -> bool {
        !article.title.is_empty()
            && !article.source_domain.is_empty()
            && !article.excerpt.is_empty()
            && Url::parse(&article.media).is_ok()
            && Url::parse(&article.link).is_ok()
    }
}

impl ArticleFilter for MalformedFilter {
    fn apply(
        _history: &[HistoricDocument],
        _stack: &[Document],
        mut articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        articles.retain(|article| MalformedFilter::is_valid(article));
        Ok(articles)
    }
}

pub(crate) struct CommonFilter;

impl ArticleFilter for CommonFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        DuplicateFilter::apply(history, stack, articles)
            .and_then(|articles| MalformedFilter::apply(history, stack, articles))
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use crate::{
        document::{document_from_article, Document},
        stack::filters::CommonFilter,
    };
    use xayn_discovery_engine_providers::Article;

    use super::*;

    #[test]
    fn test_filter_duplicate_stack() {
        let valid_articles: Vec<Article> =
            serde_json::from_str(include_str!("../../test-fixtures/articles-valid.json")).unwrap();
        assert_eq!(valid_articles.len(), 4);

        let documents = valid_articles
            .iter()
            .take(2)
            .map(|article| {
                let doc = Document::default();
                document_from_article(article.clone(), doc.stack_id, doc.smbert_embedding).unwrap()
            })
            .collect::<Vec<_>>();

        let filtered = CommonFilter::apply(&[], &documents, valid_articles)
            .unwrap()
            .into_iter()
            .map(|article| article.title)
            .collect::<Vec<_>>();

        assert_eq!(filtered, [
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
        ]);
    }

    #[test]
    fn test_filter_duplicate_history() {
        let valid_articles = serde_json::from_str::<Vec<Article>>(include_str!(
            "../../test-fixtures/articles-valid.json"
        ))
        .unwrap();
        assert_eq!(valid_articles.len(), 4);

        let history = valid_articles
            .iter()
            .take(2)
            .cloned()
            .map(TryInto::try_into)
            .collect::<Result<Vec<HistoricDocument>, _>>()
            .unwrap();

        let filtered = CommonFilter::apply(&history, &[], valid_articles)
            .unwrap()
            .into_iter()
            .map(|article| article.title)
            .collect::<Vec<_>>();

        assert_eq!(filtered, [
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
        ]);
    }

    #[test]
    fn test_filter_media() {
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

        let result = CommonFilter::apply(&[], documents.as_slice(), input).unwrap();
        let titles = result.iter().map(|a| &a.title).collect::<Vec<_>>();

        assert_eq!(titles.as_slice(), [
            "Olympic champion Lundby laments ski jumping's weight issues",
            "Jerusalem blanketed in white after rare snowfall",
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
        ]);
    }

    #[test]
    fn test_filter_title() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../test-fixtures/articles-invalid-title.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_link() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../test-fixtures/articles-invalid-link.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 3);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_excerpt() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../test-fixtures/articles-invalid-excerpt.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_clean_url() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../test-fixtures/articles-invalid-clean-url.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }
}
