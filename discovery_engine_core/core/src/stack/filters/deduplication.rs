// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details..
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::{HashMap, HashSet};
use xayn_discovery_engine_providers::Article;

use crate::document::{Document, HistoricDocument, NewsResource};

/// Normalizes `text` to a trimmed lowercase string.
fn normalize(text: &str) -> String {
    text.trim().to_lowercase()
}

pub(crate) trait Deduplicable {
    fn title(&self) -> &str;
    fn rank(&self) -> u64;
    fn url(&self) -> &str;
}

impl Deduplicable for Article {
    fn title(&self) -> &str {
        &self.title
    }

    fn rank(&self) -> u64 {
        self.rank
    }

    fn url(&self) -> &str {
        self.url.as_str()
    }
}

impl Deduplicable for Document {
    fn title(&self) -> &str {
        &self.resource.title
    }

    fn rank(&self) -> u64 {
        self.resource.rank
    }

    fn url(&self) -> &str {
        self.resource.url.as_str()
    }
}

pub(crate) struct DuplicateFilter;

impl DuplicateFilter {
    pub(crate) fn apply<T>(
        history: &[HistoricDocument],
        stack: &[Document],
        mut documents: Vec<T>,
    ) -> Vec<T>
    where
        T: Deduplicable,
    {
        // discard dups in the title keeping only the best ranked
        documents.sort_unstable_by(|art1, art2| {
            normalize(art1.title())
                .cmp(&normalize(art2.title()))
                .then(art1.rank().cmp(&art2.rank()))
        });
        documents.dedup_by_key(|art| normalize(art.title()));

        // discard dups in the link (such dups assumed to have the same rank)
        documents.sort_unstable_by(|art1, art2| art1.url().cmp(art2.url()));
        documents.dedup_by(|art1, art2| art1.url() == art2.url());

        if !history.is_empty() {
            let (hist_urls, hist_titles) = history
                .iter()
                .map(|doc| (doc.url.as_str(), normalize(&doc.title)))
                .unzip::<_, _, HashSet<_>, HashSet<_>>();

            // discard dups of historical documents
            documents.retain(|art| {
                !hist_urls.contains(art.url()) && !hist_titles.contains(&normalize(art.url()))
            });
        }

        if !stack.is_empty() {
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
            documents.retain(|art| {
                !stack_urls.contains(art.url())
                    && stack_titles
                        .get(&normalize(art.title()))
                        .map_or(true, |doc_rank| &art.rank() < doc_rank)
            });
        }

        documents
    }
}
