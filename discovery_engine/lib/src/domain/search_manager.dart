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

import 'package:xayn_discovery_engine/discovery_engine.dart'
    show cfgFeatureStorage;
import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show SearchClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, SearchFailureReason;
import 'package:xayn_discovery_engine/src/api/models/active_search.dart'
    show ActiveSearch, ActiveSearchApiConversion, SearchBy;
import 'package:xayn_discovery_engine/src/api/models/document.dart' as api;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    as domain show ActiveSearch;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/active_search_repo.dart'
    show ActiveSearchRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/engine_state_repo.dart'
    show EngineStateRepository;

typedef DocsByReaction = Map<UserReaction, List<Document>>;

/// Business logic concerning the management of the active search.
class SearchManager {
  final Engine _engine;
  final EventConfig _config;
  final ActiveSearchRepository _searchRepo;
  final DocumentRepository _docRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final EngineStateRepository _engineStateRepo;

  SearchManager(
    this._engine,
    this._config,
    this._searchRepo,
    this._docRepo,
    this._activeRepo,
    this._engineStateRepo,
  );

  /// Handle the given search client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleSearchClientEvent(SearchClientEvent event) =>
      event.maybeWhen(
        activeSearchRequested: (term, by) => activeSearchRequested(by, term),
        nextActiveSearchBatchRequested: nextActiveSearchBatchRequested,
        restoreActiveSearchRequested: restoreActiveSearchRequested,
        activeSearchClosed: activeSearchClosed,
        activeSearchTermRequested: activeSearchTermRequested,
        deepSearchRequested: deepSearchRequested,
        trendingTopicsRequested: trendingTopicsRequested,
        orElse: () =>
            throw UnimplementedError('handler not implemented for $event'),
      );

  Future<List<api.Document>> _getActiveSearchDocuments(
    domain.ActiveSearch search,
  ) async {
    final List<DocumentWithActiveData> searchDocs;

    switch (search.searchBy) {
      case SearchBy.query:
        searchDocs = await _engine.searchByQuery(
          search.searchTerm,
          search.requestedPageNb,
          search.pageSize,
        );
        break;
      case SearchBy.topic:
        searchDocs = await _engine.searchByTopic(
          search.searchTerm,
          search.requestedPageNb,
          search.pageSize,
        );
        break;
    }

    await _engineStateRepo.save(await _engine.serialize());
    await _docRepo.updateMany(searchDocs.map((e) => e.document));

    for (final docWithData in searchDocs) {
      final id = docWithData.document.documentId;
      await _activeRepo.update(id, docWithData.data);
    }

    return searchDocs
        .map((docWithData) => docWithData.document.toApiRepr())
        .toList();
  }

  /// Obtain the first batch of active search documents and persist to repositories.
  Future<EngineEvent> activeSearchRequested(
    SearchBy by,
    String term,
  ) async {
    if (cfgFeatureStorage) {
      final search = ActiveSearch(searchBy: by, searchTerm: term);
      const page = 1;
      final pageSize = _config.maxSearchDocs;
      final List<DocumentWithActiveData> docs;
      try {
        switch (search.searchBy) {
          case SearchBy.query:
            docs =
                await _engine.searchByQuery(search.searchTerm, page, pageSize);
            break;
          case SearchBy.topic:
            docs =
                await _engine.searchByTopic(search.searchTerm, page, pageSize);
            break;
        }
      } on Exception catch (e) {
        if (e.toString().contains('Search request failed: open search')) {
          return const EngineEvent.activeSearchRequestFailed(
            SearchFailureReason.openActiveSearch,
          );
        }
        rethrow;
      }
      await _engineStateRepo.save(await _engine.serialize());

      return EngineEvent.activeSearchRequestSucceeded(
        search,
        docs.map((doc) => doc.document.toApiRepr()).toList(),
      );
    }

    if (await _searchRepo.getCurrent() != null) {
      const reason = SearchFailureReason.openActiveSearch;
      return const EngineEvent.activeSearchRequestFailed(reason);
    }

    final search = domain.ActiveSearch(
      searchBy: by,
      searchTerm: term,
      requestedPageNb: 1,
      pageSize: _config.maxSearchDocs,
    );
    final docs = await _getActiveSearchDocuments(search);
    await _searchRepo.save(search);
    return EngineEvent.activeSearchRequestSucceeded(search.toApiRepr(), docs);
  }

  /// Obtain the next batch of active search documents and persist to repositories.
  Future<EngineEvent> nextActiveSearchBatchRequested() async {
    if (cfgFeatureStorage) {
      final domain.ActiveSearch search;
      final List<DocumentWithActiveData> docs;
      try {
        search = await _engine.searchedBy();
        docs = await _engine.searchNextBatch();
      } on Exception catch (e) {
        if (e.toString().contains('Search request failed: no search')) {
          return const EngineEvent.nextActiveSearchBatchRequestFailed(
            SearchFailureReason.noActiveSearch,
          );
        }
        rethrow;
      }
      await _engineStateRepo.save(await _engine.serialize());

      return EngineEvent.nextActiveSearchBatchRequestSucceeded(
        search.toApiRepr(),
        docs.map((doc) => doc.document.toApiRepr()).toList(),
      );
    }

    final search = await _searchRepo.getCurrent();

    if (search == null) {
      const reason = SearchFailureReason.noActiveSearch;
      return const EngineEvent.nextActiveSearchBatchRequestFailed(reason);
    }

    // lets update active search params
    search.requestedPageNb += 1;
    final docs = await _getActiveSearchDocuments(search);
    await _searchRepo.save(search);
    return EngineEvent.nextActiveSearchBatchRequestSucceeded(
      search.toApiRepr(),
      docs,
    );
  }

  /// Returns the list of active search documents, ordered by their global rank.
  ///
  /// That is, documents are ordered by their timestamp, then local rank.
  Future<EngineEvent> restoreActiveSearchRequested() async {
    if (cfgFeatureStorage) {
      final domain.ActiveSearch search;
      final List<DocumentWithActiveData> docs;
      try {
        search = await _engine.searchedBy();
        docs = await _engine.restoreSearch();
      } on Exception catch (e) {
        if (e.toString().contains('Search request failed: no search')) {
          return const EngineEvent.nextActiveSearchBatchRequestFailed(
            SearchFailureReason.noActiveSearch,
          );
        }
        rethrow;
      }

      if (docs.isEmpty) {
        return const EngineEvent.restoreActiveSearchFailed(
          SearchFailureReason.noResultsAvailable,
        );
      }

      return EngineEvent.restoreActiveSearchSucceeded(
        search.toApiRepr(),
        docs.map((doc) => doc.document.toApiRepr()).toList(),
      );
    }

    final search = await _searchRepo.getCurrent();

    if (search == null) {
      const reason = SearchFailureReason.noActiveSearch;
      return const EngineEvent.restoreActiveSearchFailed(reason);
    }

    final allDocs = await _docRepo.fetchAll();
    final searchDocs = allDocs
        // we only want active search documents
        .where((doc) => doc.isSearched && doc.isActive)
        .toList();

    if (searchDocs.isEmpty) {
      const reason = SearchFailureReason.noResultsAvailable;
      return const EngineEvent.restoreActiveSearchFailed(reason);
    }

    searchDocs.sort((doc1, doc2) {
      final timeOrd = doc1.timestamp.compareTo(doc2.timestamp);
      return timeOrd == 0
          ? doc1.batchIndex.compareTo(doc2.batchIndex)
          : timeOrd;
    });

    final docs = searchDocs.map((doc) => doc.toApiRepr()).toList();

    return EngineEvent.restoreActiveSearchSucceeded(search.toApiRepr(), docs);
  }

  /// Return the active search term.
  Future<EngineEvent> activeSearchTermRequested() async {
    if (cfgFeatureStorage) {
      final domain.ActiveSearch search;
      try {
        search = await _engine.searchedBy();
      } on Exception catch (e) {
        if (e.toString().contains('Search request failed: no search')) {
          return const EngineEvent.activeSearchTermRequestFailed(
            SearchFailureReason.noActiveSearch,
          );
        }
        rethrow;
      }

      return EngineEvent.activeSearchTermRequestSucceeded(search.searchTerm);
    }

    final search = await _searchRepo.getCurrent();

    if (search == null) {
      const reason = SearchFailureReason.noActiveSearch;
      return const EngineEvent.activeSearchTermRequestFailed(reason);
    }

    return EngineEvent.activeSearchTermRequestSucceeded(search.searchTerm);
  }

  /// Obtains the deep search documents related to a document.
  ///
  /// These documents aren't persisted to repositories.
  Future<EngineEvent> deepSearchRequested(DocumentId id) async {
    final doc = await _docRepo.fetchById(id);
    final data = await _activeRepo.fetchById(id);
    if (doc == null || !doc.isActive || data == null) {
      throw ArgumentError('id $id does not identify an active document');
    }
    final term = doc.resource.snippet.isNotEmpty
        ? doc.resource.snippet
        : doc.resource.title;
    final market = FeedMarket(
      langCode: doc.resource.language,
      countryCode: doc.resource.country,
    );
    final embedding = data.smbertEmbedding;

    final List<DocumentWithActiveData> docs;
    try {
      docs = await _engine.deepSearch(term, market, embedding);
    } catch (e) {
      const fewWords =
          'The sequence must contain at least `KEY_PHRASE_SIZE` valid words';
      const notFound = 'HTTP status client error (404 Not Found) for url';
      final message = e.toString();
      if (message.contains(fewWords) || message.contains(notFound)) {
        return const EngineEvent.deepSearchRequestFailed(
          SearchFailureReason.noResultsAvailable,
        );
      }
      rethrow;
    }

    if (docs.isEmpty) {
      return const EngineEvent.deepSearchRequestFailed(
        SearchFailureReason.noResultsAvailable,
      );
    }

    return EngineEvent.deepSearchRequestSucceeded(
      docs.map((doc) => doc.document.toApiRepr()).toList(),
    );
  }

  /// Return the current trending topics.
  Future<EngineEvent> trendingTopicsRequested() async {
    final topics = await _engine.getTrendingTopics();

    // TODO: do we need to persist the engine state??
    await _engineStateRepo.save(await _engine.serialize());

    if (topics.isEmpty) {
      const reason = SearchFailureReason.noResultsAvailable;
      return const EngineEvent.trendingTopicsRequestFailed(reason);
    }

    return EngineEvent.trendingTopicsRequestSucceeded(topics);
  }

  /// Clear the active search and deactivate interacted search documents.
  Future<EngineEvent> activeSearchClosed() async {
    if (await _searchRepo.getCurrent() == null) {
      const reason = SearchFailureReason.noActiveSearch;
      return const EngineEvent.activeSearchClosedFailed(reason);
    }

    await _searchRepo.clear();

    final allDocs = await _docRepo.fetchAll();
    final searchDocs = allDocs
        // we only want search documents
        .where((doc) => doc.isSearched && doc.isActive);

    if (searchDocs.isEmpty) {
      return const EngineEvent.activeSearchClosedSucceeded();
    }

    final ids = searchDocs.map((doc) => doc.documentId);
    await _activeRepo.removeByIds(ids);

    final docsByInteraction = searchDocs.fold<DocsByReaction>({}, (aggr, doc) {
      return {
        ...aggr,
        doc.userReaction: [
          ...aggr[doc.userReaction] ?? <Document>[],
          doc,
        ],
      };
    });

    // we want to leave interacted docs as part of history
    final interacted = [
      ...docsByInteraction[UserReaction.positive] ?? <Document>[],
      ...docsByInteraction[UserReaction.negative] ?? <Document>[],
    ].map((doc) => doc..isActive = false);
    await _docRepo.updateMany(interacted);

    // we can remove non interacted docs from the database
    final nonInteracted = docsByInteraction[UserReaction.neutral] ?? [];
    final nonInteractedIds = nonInteracted.map((doc) => doc.documentId).toSet();
    await _docRepo.removeByIds(nonInteractedIds);

    return const EngineEvent.activeSearchClosedSucceeded();
  }
}
