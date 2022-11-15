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

use itertools::Itertools;

use crate::document::{Document, WeightedSource};

#[allow(dead_code)]
/// Sorts `documents` by increasing order of source weight.
pub(crate) fn sort_by_source(documents: &mut [Document], sources: &[WeightedSource]) {
    documents.sort_by_cached_key(|doc| source_weight(doc, sources));
}

#[allow(dead_code)]
/// Returns the position of the document with heaviest source weight.
pub(crate) fn position_max_by_source(
    documents: &[Document],
    sources: &[WeightedSource],
) -> Option<usize> {
    documents
        .iter()
        .position_max_by_key(|doc| source_weight(doc, sources))
}

/// Source weight of a document.
pub(crate) fn source_weight(document: &Document, sources: &[WeightedSource]) -> i32 {
    let source = &document.resource.source_domain;
    sources
        .iter()
        .find_map(|weighted| (&weighted.source == source).then_some(weighted.weight))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use xayn_discovery_engine_providers::{GenericArticle, NewscatcherArticle};

    use super::*;
    use crate::document::{Document, WeightedSource};

    impl WeightedSource {
        fn new(source_domain: &str, weight: i32) -> Self {
            let source = source_domain.to_string();
            Self { source, weight }
        }
    }

    fn documents_from_json() -> Vec<Document> {
        let json = include_str!("../../../test-fixtures/articles-valid.json");
        let articles_nc: Vec<NewscatcherArticle> = serde_json::from_str(json).unwrap();
        let articles: Vec<GenericArticle> = articles_nc
            .into_iter()
            .map(|art| art.try_into().unwrap())
            .collect();

        articles
            .iter()
            .take(3)
            .map(|article| {
                let doc = Document::default();
                (article.clone(), doc.stack_id, doc.bert_embedding)
                    .try_into()
                    .unwrap()
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_position_max_by_source() {
        let mut docs = documents_from_json();
        assert_eq!(docs.len(), 3);
        docs[0].resource.source_domain = "example.com".to_string();
        docs[1].resource.source_domain = "example.org".to_string();
        docs[2].resource.source_domain = "example.net".to_string();

        let mut sources = vec![
            WeightedSource::new("example.org", -1),
            WeightedSource::new("example.net", 1),
        ];
        assert_eq!(position_max_by_source(&docs, &sources), Some(2));

        sources = vec![
            WeightedSource::new("example.com", 1),
            WeightedSource::new("example.org", 3),
            WeightedSource::new("example.net", 2),
        ];
        assert_eq!(position_max_by_source(&docs, &sources), Some(1));

        sources = vec![
            WeightedSource::new("example.org", -1),
            WeightedSource::new("example.net", -1),
        ];
        assert_eq!(position_max_by_source(&docs, &sources), Some(0));

        assert!(position_max_by_source(&[], &sources).is_none());
    }

    #[test]
    fn test_sort_by_source() {
        let mut docs = documents_from_json();
        assert_eq!(docs.len(), 3);
        docs[0].resource.source_domain = "example.com".to_string();
        docs[1].resource.source_domain = "example.org".to_string();
        docs[2].resource.source_domain = "example.net".to_string();

        let sources = vec![
            WeightedSource::new("example.org", -1),
            WeightedSource::new("example.net", 1),
        ];

        sort_by_source(&mut docs, &sources);
        assert_eq!(&docs[0].resource.source_domain, "example.org");
        assert_eq!(&docs[1].resource.source_domain, "example.com");
        assert_eq!(&docs[2].resource.source_domain, "example.net");
    }
}
