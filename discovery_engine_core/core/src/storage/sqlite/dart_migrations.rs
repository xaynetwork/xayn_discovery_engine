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
    smbert: impl Fn(&str) -> Option<Embedding> + Sync,
    data: &mut DartMigrationData,
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
        storage.search().store_new_search(&search, &[]).await?;
    }

    add_missing_embeddings(smbert, &mut data.documents);
    store_migration_document_data(storage, &data.documents).await?;

    Ok(())
}

fn add_missing_embeddings<'a>(
    smbert: impl Fn(&str) -> Option<Embedding> + Sync,
    documents: &'a mut [MigrationDocument],
) {
    for document in documents {
        if document.smbert_embedding.is_none() {
            document.smbert_embedding = smbert(document.resource.title_or_snippet());
        }
    }
}

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
                date_published: document.resource.date_published.clone(),
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
    use super::{super::setup::init_storage_system_once, *};

    #[tokio::test]
    async fn test_store_migration_data() {
        let data = DartMigrationData {
            engine_state: Some(vec![1, 2, 3, 4, 8, 7, 0]),
            trusted_sources: vec!["foo.example".into(), "bar.invalid".into()],
            excluded_sources: vec!["dodo.local".into()],
            documents: vec![],
            search: None,
        };
        let storage = init_storage_system_once(None, Some(&data)).await.unwrap();
        let engine_state = storage.state().fetch().await.unwrap();
        let trusted_sources = storage.source_preference().fetch_trusted().await.unwrap();
        let excluded_sources = storage.source_preference().fetch_excluded().await.unwrap();

        assert_eq!(engine_state, data.engine_state);
        assert_eq!(trusted_sources, data.trusted_sources.into_iter().collect());
        assert_eq!(
            excluded_sources,
            data.excluded_sources.into_iter().collect()
        );

        //FIXME test documents search, feed, with history and without history
    }
}
