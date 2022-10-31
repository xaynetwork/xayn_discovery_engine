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

import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show SearchClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, SearchFailureReason;
import 'package:xayn_discovery_engine/src/api/models/active_search.dart'
    show ActiveSearch, ActiveSearchApiConversion, SearchBy;
import 'package:xayn_discovery_engine/src/api/models/document.dart' as api;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    as domain show ActiveSearch;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

typedef DocsByReaction = Map<UserReaction, List<Document>>;

/// Business logic concerning the management of the active search.
class SearchManager {
  final Engine _engine;

  SearchManager(this._engine);

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

  /// Obtain the first batch of active search documents and persist to repositories.
  Future<EngineEvent> activeSearchRequested(SearchBy by, String term) async {
    final search = ActiveSearch(searchBy: by, searchTerm: term);
    const page = 1;
    final List<DocumentWithActiveData> docs;
    try {
      switch (search.searchBy) {
        case SearchBy.query:
          docs = await _engine.searchByQuery(search.searchTerm, page);
          break;
        case SearchBy.topic:
          docs = await _engine.searchByTopic(search.searchTerm, page);
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

    return EngineEvent.activeSearchRequestSucceeded(
      search,
      docs.map((doc) => doc.document.toApiRepr()).toList(),
    );
  }

  /// Obtain the next batch of active search documents and persist to repositories.
  Future<EngineEvent> nextActiveSearchBatchRequested() async {
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

    return EngineEvent.nextActiveSearchBatchRequestSucceeded(
      search.toApiRepr(),
      docs.map((doc) => doc.document.toApiRepr()).toList(),
    );
  }

  /// Returns the list of active search documents, ordered by their global rank.
  ///
  /// That is, documents are ordered by their timestamp, then local rank.
  Future<EngineEvent> restoreActiveSearchRequested() async {
    final domain.ActiveSearch search;
    final List<DocumentWithActiveData> docs;
    try {
      search = await _engine.searchedBy();
      docs = await _engine.restoreSearch();
    } on Exception catch (e) {
      if (e.toString().contains('Search request failed: no search')) {
        return const EngineEvent.restoreActiveSearchFailed(
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

  /// Return the active search term.
  Future<EngineEvent> activeSearchTermRequested() async {
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

  /// Obtains the deep search documents related to a document.
  ///
  /// These documents aren't persisted to repositories.
  Future<EngineEvent> deepSearchRequested(DocumentId id) async {
    final List<DocumentWithActiveData> docs;
    try {
      docs = await _engine.searchById(id);
    } on Exception catch (e) {
      final message = e.toString();
      if (message.contains('Search request failed: no document')) {
        throw ArgumentError('id $id does not identify an active document');
      }
      if (message.contains(
            'The sequence must contain at least `KEY_PHRASE_SIZE` valid words',
          ) ||
          message
              .contains('HTTP status client error (404 Not Found) for url')) {
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
    final topics = await _engine.trendingTopics();

    if (topics.isEmpty) {
      const reason = SearchFailureReason.noResultsAvailable;
      return const EngineEvent.trendingTopicsRequestFailed(reason);
    }

    return EngineEvent.trendingTopicsRequestSucceeded(topics);
  }

  /// Clear the active search and deactivate interacted search documents.
  Future<EngineEvent> activeSearchClosed() async {
    try {
      await _engine.closeSearch();
    } on Exception catch (e) {
      if (e.toString().contains('Search request failed: no search')) {
        return const EngineEvent.activeSearchClosedFailed(
          SearchFailureReason.noActiveSearch,
        );
      }
      rethrow;
    }

    return const EngineEvent.activeSearchClosedSucceeded();
  }
}
