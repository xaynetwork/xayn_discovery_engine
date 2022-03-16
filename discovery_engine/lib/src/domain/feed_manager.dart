// Copyright 2021 Xayn AG
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
    show FeedClientEvent;
import 'package:xayn_discovery_engine/src/api/events/engine_events.dart'
    show EngineEvent, FeedFailureReason;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/event_handler.dart'
    show EventConfig;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/engine_state_repo.dart'
    show EngineStateRepository;
import 'package:xayn_discovery_engine/src/domain/repository/excluded_sources_repo.dart'
    show ExcludedSourcesRepository;

/// Business logic concerning the management of the feed.
class FeedManager {
  final Engine _engine;
  final EventConfig _config;
  final DocumentRepository _docRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final ChangedDocumentRepository _changedRepo;
  final EngineStateRepository _engineStateRepo;
  final ExcludedSourcesRepository _excludedSourcesRepository;

  FeedManager(
    this._engine,
    this._config,
    this._docRepo,
    this._activeRepo,
    this._changedRepo,
    this._engineStateRepo,
    this._excludedSourcesRepository,
  );

  /// Handle the given feed client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleFeedClientEvent(FeedClientEvent event) =>
      event.maybeWhen(
        restoreFeedRequested: restoreFeed,
        nextFeedBatchRequested: nextFeedBatch,
        feedDocumentsClosed: deactivateDocuments,
        excludedSourceAdded: addExcludedSource,
        excludedSourceRemoved: removeExcludedSource,
        excludedSourcesListRequested: getExcludedSourcesList,
        orElse: () =>
            throw UnimplementedError('handler not implemented for $event'),
      );

  /// Generates the feed of active documents, ordered by their global rank.
  ///
  /// That is, documents are ordered by their timestamp, then local rank.
  Future<EngineEvent> restoreFeed() => _docRepo.fetchAll().then(
        (docs) {
          final sortedActives = docs
            ..retainWhere(
              (doc) =>
                  // we only want active documents
                  doc.isActive &&
                  // we only want feed documents (isSearched == false)
                  doc.isSearched == false,
            )
            ..sort((doc1, doc2) {
              final timeOrd = doc1.timestamp.compareTo(doc2.timestamp);
              return timeOrd == 0
                  ? doc1.batchIndex.compareTo(doc2.batchIndex)
                  : timeOrd;
            });

          final feed = sortedActives.map((doc) => doc.toApiDocument()).toList();
          return EngineEvent.restoreFeedSucceeded(feed);
        },
      );

  /// Obtain the next batch of feed documents and persist to repositories.
  Future<EngineEvent> nextFeedBatch() async {
    final history = await _docRepo.fetchHistory();
    final List<DocumentWithActiveData> feedDocs;
    try {
      feedDocs = await _engine.getFeedDocuments(history, _config.maxFeedDocs);
    } catch (e) {
      return EngineEvent.nextFeedBatchRequestFailed(
        FeedFailureReason.stacksOpsError,
        errors: '$e',
      );
    }

    await _engineStateRepo.save(await _engine.serialize());

    await _docRepo.updateMany(feedDocs.map((e) => e.document));
    for (final docWithData in feedDocs) {
      final id = docWithData.document.documentId;
      await _activeRepo.update(id, docWithData.data);
    }

    final feed = feedDocs
        .map((docWithData) => docWithData.document.toApiDocument())
        .toList();

    if (feed.isEmpty) {
      const reason = FeedFailureReason.noNewsForMarket;
      return const EngineEvent.nextFeedBatchRequestFailed(reason);
    }
    return EngineEvent.nextFeedBatchRequestSucceeded(feed);
  }

  /// Deactivate the given documents.
  Future<EngineEvent> deactivateDocuments(Set<DocumentId> ids) async {
    await _activeRepo.removeByIds(ids);
    await _changedRepo.removeMany(ids);

    final docs = await _docRepo.fetchByIds(ids);
    final inactives = docs.map((doc) => doc..isActive = false);
    await _docRepo.updateMany(inactives);

    return const EngineEvent.clientEventSucceeded();
  }

  Future<void> _throwIfInvalidSource(String source) async {
    if (source.isEmpty) {
      throw ArgumentError('source can\'t be empty');
    }

    final allDocuments = await _docRepo.fetchAll();
    final doesExist = allDocuments
        .where((doc) => doc.isActive)
        .map((doc) => doc.resource.sourceDomain)
        .toSet()
        .contains(source);

    if (!doesExist) {
      throw ArgumentError('source $source not found in database');
    }
  }

  /// Adds a source to excluded sources set.
  Future<EngineEvent> addExcludedSource(String source) async {
    await _throwIfInvalidSource(source);
    final sources = await _excludedSourcesRepository.getAll();
    sources.add(source);
    await _excludedSourcesRepository.save(sources);
    // TODO: send updated sources to the engine
    return const EngineEvent.clientEventSucceeded();
  }

  /// Removes a source to excluded sources set.
  Future<EngineEvent> removeExcludedSource(String source) async {
    await _throwIfInvalidSource(source);
    final sources = await _excludedSourcesRepository.getAll();
    sources.remove(source);
    await _excludedSourcesRepository.save(sources);
    // TODO: send updated sources to the engine
    return const EngineEvent.clientEventSucceeded();
  }

  /// Returns excluded sources.
  Future<EngineEvent> getExcludedSourcesList() async {
    final sources = await _excludedSourcesRepository.getAll();
    return EngineEvent.excludedSourcesListRequestSucceeded(sources);
  }
}
