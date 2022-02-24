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
    show EngineEvent, EngineExceptionReason, FeedFailureReason;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarkets;
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

/// Business logic concerning the management of the feed.
class FeedManager {
  final Engine _engine;
  int _maxDocs;
  final DocumentRepository _docRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final ChangedDocumentRepository _changedRepo;
  final EngineStateRepository _engineStateRepo;

  FeedManager(
    this._engine,
    this._maxDocs,
    this._docRepo,
    this._activeRepo,
    this._changedRepo,
    this._engineStateRepo,
  );

  /// Handle the given feed client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  Future<EngineEvent> handleFeedClientEvent(FeedClientEvent event) =>
      event.maybeWhen(
        configurationChanged: (final feedMarkets, final maxItemsPerFeedBatch) =>
            changeConfiguration(feedMarkets, maxItemsPerFeedBatch),
        feedRequested: () => restoreFeed(),
        nextFeedBatchRequested: () => nextFeedBatch(),
        feedDocumentsClosed: (ids) => deactivateDocuments(ids),
        orElse: () =>
            throw UnimplementedError('handler not implemented for $event'),
      );

  /// Changes the configuration of the feed.
  Future<EngineEvent> changeConfiguration(
    final FeedMarkets? feedMarkets,
    final int? maxItemsPerFeedBatch,
  ) async {
    if (feedMarkets != null) {
      final history = await _docRepo.fetchHistory();
      try {
        await _engine.setMarkets(history, feedMarkets);
      } catch (e, st) {
        return EngineEvent.engineExceptionRaised(
          EngineExceptionReason.genericError,
          message: '$e',
          stackTrace: '$st',
        );
      }
    }
    if (maxItemsPerFeedBatch != null) {
      _maxDocs = maxItemsPerFeedBatch;
    }

    return const EngineEvent.clientEventSucceeded();
  }

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
          return EngineEvent.feedRequestSucceeded(feed);
        },
      );

  /// Obtain the next batch of feed documents and persist to repositories.
  Future<EngineEvent> nextFeedBatch() async {
    final history = await _docRepo.fetchHistory();
    final feedDocs = await _engine.getFeedDocuments(history, _maxDocs);
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
}
