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

use url::Url;

use crate::{
    document::{Document, HistoricDocument},
    stack::filters::DuplicateFilter,
};
use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_providers::Article;

pub(crate) trait ArticleFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError>;
}

pub(crate) struct MalformedFilter;

impl MalformedFilter {
    fn is_valid(article: &Article) -> bool {
        !article.title.is_empty()
            && !article.source_domain.is_empty()
            && !article.excerpt.is_empty()
            && Url::parse(&article.link).is_ok()
    }
}

impl ArticleFilter for MalformedFilter {
    fn apply(
        _history: &[HistoricDocument],
        _stack: &[Document],
        mut articles: Vec<Article>,
    ) -> Result<Vec<Article>, GenericError> {
        articles.retain(MalformedFilter::is_valid);
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
        MalformedFilter::apply(history, stack, articles)
            .map(|articles| DuplicateFilter::apply(history, stack, articles))
    }
}

pub(crate) struct SourcesFilter;

impl SourcesFilter {
    /// Discard articles with an excluded source domain.
    pub(crate) fn apply(mut articles: Vec<Article>, excluded_sources: &[String]) -> Vec<Article> {
        articles.retain(|art| !excluded_sources.contains(&art.source_domain));
        articles
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, iter::FromIterator};

    use crate::document::Document;
    use itertools::Itertools;
    use xayn_discovery_engine_providers::Article;

    use super::*;

    #[test]
    fn test_filter_duplicate_stack() {
        let valid_articles: Vec<Article> =
            serde_json::from_str(include_str!("../../../test-fixtures/articles-valid.json"))
                .unwrap();
        assert_eq!(valid_articles.len(), 4);

        let documents = valid_articles
            .iter()
            .take(2)
            .map(|article| {
                let doc = Document::default();
                (article.clone(), doc.stack_id, doc.smbert_embedding)
                    .try_into()
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let filtered = CommonFilter::apply(&[], &documents, valid_articles)
            .unwrap()
            .into_iter()
            .map(|article| article.title)
            .sorted()
            .collect::<Vec<_>>();

        assert_eq!(filtered, [
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
        ]);
    }

    #[test]
    fn test_filter_duplicate_history() {
        let valid_articles = serde_json::from_str::<Vec<Article>>(include_str!(
            "../../../test-fixtures/articles-valid.json"
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
            .sorted()
            .collect::<Vec<_>>();

        assert_eq!(filtered, [
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
            "Porsche entwickelt Antrieb, der E-Mobilit\u{00e4}t teilweise \u{00fc}berlegen ist",
        ]);
    }

    #[test]
    fn test_filter_title() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../../test-fixtures/articles-invalid-title.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_link() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../../test-fixtures/articles-invalid-link.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 3);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_excerpt() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../../test-fixtures/articles-invalid-excerpt.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_clean_url() {
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../../test-fixtures/articles-invalid-clean-url.json"
        ))
        .unwrap();
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_sources() {
        let articles: Vec<Article> =
            serde_json::from_str(include_str!("../../../test-fixtures/articles-valid.json"))
                .unwrap();
        assert_eq!(articles.len(), 4);

        // all 4 articles have source domain example.com
        let filtered1 = SourcesFilter::apply(articles, &["example.org".to_string()]);
        assert_eq!(filtered1.len(), 4);

        let filtered2 = SourcesFilter::apply(filtered1, &["example.com".to_string()]);
        assert!(filtered2.is_empty());
    }

    #[test]
    fn test_dedup_articles_them_self() {
        let valid_articles = serde_json::from_str::<Vec<Article>>(include_str!(
            "../../../test-fixtures/articles-valid.json"
        ))
        .unwrap();
        assert!(valid_articles.len() >= 4);

        // start with 4 articles {0, 1, 2, 3}
        let mut articles = valid_articles.clone();

        // add some more: {0, 1, 2, 3, 0, 1', 2', 3'}
        articles.push(valid_articles[0].clone());
        articles.push({
            let mut article = valid_articles[1].clone();
            article.link = "https://with_same_link.test".to_owned();
            article.rank = u64::MAX;
            article
        });
        articles.push({
            let mut article = valid_articles[2].clone();
            article.title = "With same url".to_owned();
            article
        });
        articles.push({
            let mut article = valid_articles[3].clone();
            article.link = "https://unique.test".to_owned();
            article.title = "Unique".to_owned();
            article
        });

        // after filtering: {0, 1, 2/2', 3, 3'}
        let filtered = CommonFilter::apply(&[], &[], articles)
            .unwrap()
            .into_iter()
            .map(|article| (article.title, article.rank))
            .collect::<Vec<_>>();

        assert_eq!(filtered.len(), 5, "Unexpected len for: {:?}", filtered);

        let filtered = HashMap::<_, _>::from_iter(filtered);
        assert_eq!(
            filtered.get(&valid_articles[0].title),
            Some(&valid_articles[0].rank)
        );
        assert_eq!(
            filtered.get(&valid_articles[1].title),
            Some(&valid_articles[1].rank)
        );
        assert_eq!(
            filtered
                .get("With same url")
                .xor(filtered.get(&valid_articles[2].title)),
            Some(&valid_articles[2].rank)
        );
        assert_eq!(
            filtered.get(&valid_articles[3].title),
            Some(&valid_articles[3].rank)
        );
        assert_eq!(filtered.get("Unique"), Some(&valid_articles[3].rank));
    }
}
