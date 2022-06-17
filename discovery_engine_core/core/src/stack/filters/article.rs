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

use crate::{
    document::{Document, HistoricDocument},
    stack::filters::DuplicateFilter,
};
use xayn_discovery_engine_ai::GenericError;
use xayn_discovery_engine_providers::GenericArticle;

pub(crate) trait ArticleFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<GenericArticle>,
    ) -> Result<Vec<GenericArticle>, GenericError>;
}

pub(crate) struct MalformedFilter;

impl MalformedFilter {
    fn is_valid(article: &GenericArticle) -> bool {
        !article.title.is_empty() && !article.snippet.is_empty()
    }
}

impl ArticleFilter for MalformedFilter {
    fn apply(
        _history: &[HistoricDocument],
        _stack: &[Document],
        mut articles: Vec<GenericArticle>,
    ) -> Result<Vec<GenericArticle>, GenericError> {
        articles.retain(MalformedFilter::is_valid);
        Ok(articles)
    }
}

pub(crate) struct CommonFilter;

impl ArticleFilter for CommonFilter {
    fn apply(
        history: &[HistoricDocument],
        stack: &[Document],
        articles: Vec<GenericArticle>,
    ) -> Result<Vec<GenericArticle>, GenericError> {
        MalformedFilter::apply(history, stack, articles)
            .map(|articles| DuplicateFilter::apply(history, stack, articles))
    }
}

pub(crate) struct SourcesFilter;

impl SourcesFilter {
    /// Discard articles with an excluded source domain.
    pub(crate) fn apply(
        mut articles: Vec<GenericArticle>,
        excluded_sources: &[String],
    ) -> Vec<GenericArticle> {
        articles.retain(|art| !excluded_sources.contains(&art.source_domain()));
        articles
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, iter::FromIterator};

    use crate::document::Document;
    use itertools::Itertools;

    use xayn_discovery_engine_providers::NewscatcherArticle;

    use super::*;

    pub(crate) fn to_generic_article(articles: Vec<NewscatcherArticle>) -> Vec<GenericArticle> {
        articles
            .into_iter()
            .map(|x| x.try_into().unwrap())
            .collect()
    }

    fn load_articles_from_json(json: &'static str) -> Vec<GenericArticle> {
        let articles: Vec<NewscatcherArticle> = serde_json::from_str(json).unwrap();
        to_generic_article(articles)
    }

    #[test]
    fn test_filter_duplicate_stack() {
        let valid_articles =
            load_articles_from_json(include_str!("../../../test-fixtures/articles-valid.json"));
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
        let valid_articles =
            load_articles_from_json(include_str!("../../../test-fixtures/articles-valid.json"));
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
        let malformed_articles = load_articles_from_json(include_str!(
            "../../../test-fixtures/articles-invalid-title.json"
        ));
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_excerpt() {
        let malformed_articles = load_articles_from_json(include_str!(
            "../../../test-fixtures/articles-invalid-excerpt.json"
        ));
        assert_eq!(malformed_articles.len(), 2);

        let result = CommonFilter::apply(&[], &[], malformed_articles).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_sources() {
        let articles =
            load_articles_from_json(include_str!("../../../test-fixtures/articles-valid.json"));
        assert_eq!(articles.len(), 4);

        // all 4 articles have source domain example.com
        let filtered1 = SourcesFilter::apply(articles, &["example.org".to_string()]);
        assert_eq!(filtered1.len(), 4);

        let filtered2 = SourcesFilter::apply(filtered1, &["example.com".to_string()]);
        assert!(filtered2.is_empty());
    }

    #[test]
    fn test_dedup_articles_them_self() {
        let valid_articles =
            load_articles_from_json(include_str!("../../../test-fixtures/articles-valid.json"));
        assert_eq!(valid_articles.len(), 4);

        // start with 4 articles {0, 1, 2, 3}
        let mut articles = valid_articles.clone();

        // add some more: {0, 1, 2, 3, 0, 1', 2', 3'}
        articles.push(valid_articles[0].clone());
        articles.push({
            let mut article = valid_articles[1].clone();
            article.url = "https://with_same_link.test".try_into().unwrap();
            article.set_rank(u64::MAX);
            article
        });
        articles.push({
            let mut article = valid_articles[2].clone();
            article.title = "With same url".to_owned();
            article
        });
        articles.push({
            let mut article = valid_articles[3].clone();
            article.url = "https://unique.test".try_into().unwrap();
            article.title = "Unique".to_owned();
            article
        });

        // after filtering: {0, 1, 2/2', 3, 3'}
        let filtered = CommonFilter::apply(&[], &[], articles)
            .unwrap()
            .into_iter()
            .map(|article| {
                let rank = article.rank();
                (article.title, rank)
            })
            .collect::<Vec<_>>();

        assert_eq!(filtered.len(), 5, "Unexpected len for: {:?}", filtered);

        let filtered = HashMap::<_, _>::from_iter(filtered);
        assert_eq!(
            filtered.get(&valid_articles[0].title),
            Some(&valid_articles[0].rank())
        );
        assert_eq!(
            filtered.get(&valid_articles[1].title),
            Some(&valid_articles[1].rank())
        );
        assert_eq!(
            filtered
                .get("With same url")
                .xor(filtered.get(&valid_articles[2].title)),
            Some(&valid_articles[2].rank())
        );
        assert_eq!(
            filtered.get(&valid_articles[3].title),
            Some(&valid_articles[3].rank())
        );
        assert_eq!(filtered.get("Unique"), Some(&valid_articles[3].rank()));
    }
}
