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

use std::collections::{HashMap, HashSet};

use url::Url;

use crate::{
    document::{Document, HistoricDocument, NewsResource},
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
        // discard dups in the title keeping only the best ranked
        articles.sort_unstable_by(|art1, art2| {
            normalize(&art1.title)
                .cmp(&normalize(&art2.title))
                .then(art1.rank.cmp(&art2.rank))
        });
        articles.dedup_by_key(|art| normalize(&art.title));

        // discard dups in the link (such dups assumed to have the same rank)
        articles.sort_unstable_by(|art1, art2| art1.link.cmp(&art2.link));
        articles.dedup_by(|art1, art2| art1.link == art2.link);

        let (hist_urls, hist_titles) = history
            .iter()
            .map(|doc| (doc.url.as_str(), normalize(&doc.title)))
            .unzip::<_, _, HashSet<_>, HashSet<_>>();

        // discard dups of historical documents
        articles.retain(|art| {
            !hist_urls.contains(art.link.as_str()) && !hist_titles.contains(&normalize(&art.title))
        });

        let stack_urls = stack
            .iter()
            .map(|doc| doc.resource.url.as_str())
            .collect::<HashSet<_>>();

        let stack_titles = stack
            .iter()
            .map(|doc| {
                let NewsResource { title, rank, .. } = &doc.resource;
                (normalize(title), *rank)
            })
            .fold(HashMap::new(), |mut titles, (title, rank)| {
                titles
                    .entry(title)
                    .and_modify(|best_rank| {
                        if rank < *best_rank {
                            *best_rank = rank;
                        }
                    })
                    .or_insert(rank);
                titles
            });

        // discard worse-ranked dups of stack documents; more precisely, discard:
        // * dups of stack documents in the url
        // * dups of stack documents in the title when the rank is no better
        articles.retain(|art| {
            !stack_urls.contains(art.link.as_str())
                && stack_titles
                    .get(&normalize(&art.title))
                    .map_or(true, |doc_rank| &art.rank < doc_rank)
        });

        Ok(articles)
    }
}

/// Normalizes `text` to a trimmed lowercase string.
pub(crate) fn normalize(text: &str) -> String {
    text.trim().to_lowercase()
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
            .and_then(|articles| DuplicateFilter::apply(history, stack, articles))
    }
}

pub(crate) struct SourcesFilter;

impl SourcesFilter {
    #[allow(unused)]
    /// Filter out any articles with an excluded source domain.
    fn apply(articles: Vec<Article>, excluded_sources: &[String]) -> Vec<Article> {
        articles
            .into_iter()
            .filter(|art| !excluded_sources.contains(&art.source_domain))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::TryInto, iter::FromIterator};

    use crate::document::{document_from_article, Document};
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
                document_from_article(article.clone(), doc.stack_id, doc.smbert_embedding).unwrap()
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
    fn test_filter_media() {
        let documents: Vec<Document> = vec![];
        let valid_articles: Vec<Article> =
            serde_json::from_str(include_str!("../../../test-fixtures/articles-valid.json"))
                .unwrap();
        let malformed_articles: Vec<Article> = serde_json::from_str(include_str!(
            "../../../test-fixtures/articles-some-malformed-media-urls.json"
        ))
        .unwrap();

        let input: Vec<Article> = valid_articles
            .iter()
            .cloned()
            .chain(malformed_articles.iter().cloned())
            .collect();

        let result = CommonFilter::apply(&[], documents.as_slice(), input).unwrap();
        let titles = result.iter().map(|a| &a.title).sorted().collect::<Vec<_>>();

        assert_eq!(titles.as_slice(), [
            "Jerusalem blanketed in white after rare snowfall",
            "Mensch mit d\u{00fc}sterer Prognose: \"Kollektiv versagt!\" N\u{00e4}chste Pandemie wird schlimmer als Covid-19",
            "Olympic champion Lundby laments ski jumping's weight issues",
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
