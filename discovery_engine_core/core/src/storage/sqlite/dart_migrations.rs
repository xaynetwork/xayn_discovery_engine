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

//! Module for handling dart->rust/sqltie migrations

use itertools::{chain, Itertools};
use sqlx::QueryBuilder;
use xayn_discovery_engine_ai::Embedding;
use xayn_discovery_engine_providers::Market;

use crate::{
    document::ViewMode,
    stack,
    storage::{
        models::{NewDocument, NewsResource, NewscatcherData},
        Error,
        Storage,
    },
    storage2::{DartMigrationData, MigrationDocument},
};

use super::SqliteStorage;

/// Add the data from the  dart->rust/sqltie migration to the prepared database.
pub(super) async fn store_migration_data(
    storage: &mut SqliteStorage,
    data: &mut DartMigrationData,
    smbert: &(impl Fn(&str) -> Option<Embedding> + Sync),
) -> Result<(), Error> {
    // it's okay to not have an transaction across the various migrations:
    // 1. by taking `&mut SqliteStorage` we know we have exclusive access
    // 2. databases of failed migrations should be discarded at some point
    // 3. even if the database is not discarded the db is still in a sound state,
    //    just with some history/config/preference or similar missing

    if let Some(engine_state) = &data.engine_state {
        storage.state().store(engine_state).await?;
    }

    storage
        .source_preference()
        .set_trusted(&data.trusted_sources.iter().map_into().collect())
        .await?;

    storage
        .source_preference()
        .set_excluded(&data.excluded_sources.iter().map_into().collect())
        .await?;

    if let Some(search) = &data.search {
        storage.search().store_new_search(search, &[]).await?;
    }

    add_missing_embeddings(smbert, &mut data.documents);
    store_migration_document_data(storage, &data.documents).await?;

    Ok(())
}

fn add_missing_embeddings(
    smbert: impl Fn(&str) -> Option<Embedding> + Sync,
    documents: &mut [MigrationDocument],
) {
    for document in documents {
        if document.smbert_embedding.is_none() {
            document.smbert_embedding = smbert(document.resource.title_or_snippet());
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn store_migration_document_data(
    storage: &mut SqliteStorage,
    documents: &[MigrationDocument],
) -> Result<(), Error> {
    let documents_iter = documents.iter().filter(|d| d.smbert_embedding.is_some());
    let new_documents = documents_iter.clone().map(|document| {
        NewDocument {
            id: document.id,
            news_resource: NewsResource {
                title: document.resource.title.clone(),
                snippet: document.resource.snippet.clone(),
                topic: document.resource.topic.clone(),
                url: document.resource.url.clone(),
                image: document.resource.image.clone(),
                date_published: document.resource.date_published,
                source: document.resource.source_domain.clone(),
                market: Market::new(&document.resource.language, &document.resource.country),
            },
            newscatcher_data: NewscatcherData   {
                domain_rank: document.resource.rank,
                score: document.resource.score,
            },
            embedding: document.smbert_embedding.clone().unwrap(/*we filter out all docs where it's none*/),
        }
    }).collect::<Vec<_>>();

    let mut tx = storage.pool.begin().await?;

    SqliteStorage::store_new_documents(&mut tx, &new_documents).await?;

    let mut builder = QueryBuilder::new("INSERT INTO ");

    let feed_documents = documents_iter
        .clone()
        .filter(|doc| doc.is_active && !doc.is_searched && doc.stack_id != stack::Id::nil());

    builder
        .reset()
        .push("FeedDocument(documentId) ")
        .push_values(feed_documents, |mut query, doc| {
            query.push_bind(doc.id);
        })
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    let stack_documents = documents_iter
        .clone()
        .filter(|doc| !doc.is_searched && doc.stack_id != stack::Id::nil());

    builder
        .reset()
        .push("StackDocument(documentId, stackId) ")
        .push_values(stack_documents, |mut query, doc| {
            query.push_bind(doc.id);
            query.push_bind(doc.stack_id);
        })
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    let search_documents = documents_iter
        .clone()
        .filter(|doc| doc.is_active && doc.is_searched && doc.stack_id == stack::Id::nil());

    builder
        .reset()
        .push("SearchDocument(documentId) ")
        .push_values(search_documents, |mut query, doc| {
            query.push_bind(doc.id);
        })
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    //we can only have reacted if we have seen the document
    let reacted_documents = documents_iter.clone().filter(|doc| doc.has_view_time());

    builder
        .reset()
        .push("UserReaction(documentId, userReaction) ")
        .push_values(reacted_documents, |mut query, doc| {
            query.push_bind(doc.id);
            query.push_bind(doc.reaction as u32);
        })
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    let viewed_documents = documents_iter.clone().filter(|doc| doc.has_view_time());
    let view_info = chain!(
        viewed_documents.clone().filter_map(|doc| {
            doc.story_view_time
                .map(|time| (doc.id, ViewMode::Story, time))
        }),
        viewed_documents.clone().filter_map(|doc| {
            doc.reader_view_time
                .map(|time| (doc.id, ViewMode::Reader, time))
        }),
        viewed_documents
            .filter_map(|doc| { doc.web_view_time.map(|time| (doc.id, ViewMode::Web, time)) })
    );

    builder
        .reset()
        .push("ViewTimes(documentId, viewMode, viewTimeMs) ")
        .push_values(view_info, |mut query, (id, mode, time)| {
            query.push_bind(id);
            query.push_bind(mode as u32);
            query.push_bind(u32::try_from(time.as_millis()).ok().unwrap_or(u32::MAX));
        })
        .build()
        .persistent(false)
        .execute(&mut tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

impl MigrationDocument {
    fn has_view_time(&self) -> bool {
        !(self.web_view_time.map_or(true, |d| d.is_zero())
            & self.reader_view_time.map_or(true, |d| d.is_zero())
            & self.story_view_time.map_or(true, |d| d.is_zero()))
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp::Ord, time::Duration};

    use chrono::Utc;
    use ndarray::arr1;

    use crate::{
        document::{self, UserReaction},
        storage::models::{ApiDocumentView, Paging, Search, SearchBy},
    };

    use super::{super::setup::init_storage_system_once, *};

    macro_rules! assert_migration_doc_eq_api_doc {
        ($migration_doc:expr, $search_doc:expr) => {{
            use $crate::document::UserReaction;

            let migration_doc = &$migration_doc;
            let search_doc = &$search_doc;

            assert_eq!(migration_doc.id, search_doc.document_id);
            assert_eq!(
                migration_doc.stack_id,
                search_doc.stack_id.unwrap_or_default()
            );
            assert_eq!(migration_doc.resource.title, search_doc.news_resource.title);
            assert_eq!(
                migration_doc.resource.snippet,
                search_doc.news_resource.snippet
            );
            assert_eq!(
                migration_doc.resource.date_published,
                search_doc.news_resource.date_published
            );
            assert_eq!(migration_doc.resource.image, search_doc.news_resource.image);
            assert_eq!(
                migration_doc.resource.language,
                search_doc.news_resource.market.lang_code
            );
            assert_eq!(
                migration_doc.resource.country,
                search_doc.news_resource.market.country_code
            );
            assert_eq!(
                migration_doc.resource.source_domain,
                search_doc.news_resource.source
            );
            assert_eq!(migration_doc.resource.topic, search_doc.news_resource.topic);
            assert_eq!(migration_doc.resource.url, search_doc.news_resource.url);
            assert_eq!(
                migration_doc.resource.rank,
                search_doc.newscatcher_data.domain_rank
            );
            assert_eq!(
                migration_doc.resource.score,
                search_doc.newscatcher_data.score
            );
            match search_doc.user_reaction {
                Some(reaction) => assert_eq!(migration_doc.reaction, reaction),
                None => {
                    assert_eq!(migration_doc.reaction, UserReaction::Neutral);
                    assert!((migration_doc.web_view_time.unwrap_or_default()
                        + migration_doc.story_view_time.unwrap_or_default()
                        + migration_doc.reader_view_time.unwrap_or_default())
                    .is_zero());
                }
            }
        }};
    }

    #[tokio::test]
    async fn test_store_migration_data() {
        let mut data = DartMigrationData {
            engine_state: Some(vec![1, 2, 3, 4, 8, 7, 0]),
            trusted_sources: vec!["foo.example".into(), "bar.invalid".into()],
            excluded_sources: vec!["dodo.local".into()],
            search: Some(Search {
                search_by: SearchBy::Query,
                search_term: "foo bar".into(),
                paging: Paging {
                    size: 123,
                    next_page: 312,
                },
            }),
            documents: vec![
                MigrationDocument {
                    id: document::Id::new(),
                    stack_id: stack::PersonalizedNews::id(),
                    smbert_embedding: Some(Embedding::from(arr1(&[0.0, 1.2, 3.1, 0.4]))),
                    reaction: UserReaction::Positive,
                    resource: document::NewsResource::default(),
                    is_active: true,
                    is_searched: false,
                    batch_index: 1,
                    timestamp: Utc::now(),
                    story_view_time: Some(Duration::new(3, 4)),
                    web_view_time: None,
                    reader_view_time: Some(Duration::new(5, 6)),
                },
                MigrationDocument {
                    id: document::Id::new(),
                    stack_id: stack::PersonalizedNews::id(),
                    smbert_embedding: Some(Embedding::from(arr1(&[1.0, 1.3, 8.1, 0.4]))),
                    reaction: UserReaction::Positive,
                    resource: document::NewsResource::default(),
                    is_active: false,
                    is_searched: false,
                    batch_index: 1,
                    timestamp: Utc::now(),
                    story_view_time: None,
                    web_view_time: None,
                    reader_view_time: Some(Duration::new(5, 6)),
                },
                MigrationDocument {
                    id: document::Id::new(),
                    stack_id: stack::Id::nil(),
                    smbert_embedding: Some(Embedding::from(arr1(&[0.0, 1.2, 3.1, 0.4]))),
                    reaction: UserReaction::Negative,
                    resource: document::NewsResource::default(),
                    is_active: true,
                    is_searched: true,
                    batch_index: 1,
                    timestamp: Utc::now(),
                    story_view_time: Some(Duration::new(3, 4)),
                    web_view_time: Some(Duration::new(30, 40)),
                    reader_view_time: Some(Duration::new(5, 6)),
                },
                MigrationDocument {
                    id: document::Id::new(),
                    stack_id: stack::Id::nil(),
                    smbert_embedding: None,
                    reaction: UserReaction::Neutral,
                    resource: document::NewsResource::default(),
                    is_active: false,
                    is_searched: true,
                    batch_index: 1,
                    timestamp: Utc::now(),
                    story_view_time: None,
                    web_view_time: None,
                    reader_view_time: None,
                },
            ],
        };

        let storage = init_storage_system_once(None, Some(&mut data), &|_| {
            Some(Embedding::from(arr1(&[3., 2., 1.])))
        })
        .await
        .unwrap();

        let engine_state = storage.state().fetch().await.unwrap();
        let trusted_sources = storage.source_preference().fetch_trusted().await.unwrap();
        let excluded_sources = storage.source_preference().fetch_excluded().await.unwrap();
        let (search, search_docs) = storage.search().fetch().await.unwrap();

        assert_eq!(engine_state, data.engine_state);
        assert_eq!(trusted_sources, data.trusted_sources.into_iter().collect());
        assert_eq!(
            excluded_sources,
            data.excluded_sources.into_iter().collect()
        );
        assert_eq!(
            data.documents[3].smbert_embedding.as_ref(),
            Some(&Embedding::from(arr1(&[3., 2., 1.])))
        );
        assert_eq!(Some(search), data.search);
        assert_compare_migration_and_search_documents(&data.documents, search_docs);

        //TODO[pmk] test view times
        //TODO[pmk] test feed docs
        //TODO[pmk] test history
    }

    fn assert_compare_migration_and_search_documents(
        migration_docs: &[MigrationDocument],
        mut search_docs: Vec<ApiDocumentView>,
    ) {
        let mut migration_docs = migration_docs
            .iter()
            .filter(|d| d.is_searched && d.is_active && d.stack_id == stack::Id::nil())
            .collect::<Vec<_>>();

        migration_docs.sort_by(|l, r| l.id.cmp(&r.id));
        search_docs.sort_by(|l, r| l.document_id.cmp(&r.document_id));

        for (migration_doc, search_doc) in migration_docs.iter().zip(search_docs.iter()) {
            assert_migration_doc_eq_api_doc!(migration_doc, search_doc);
        }

        assert_eq!(migration_docs.len(), search_docs.len());
    }
}
