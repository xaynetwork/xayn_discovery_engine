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
    show FeedClientEvent, FeedRequested;
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;

/// Business logic concerning the management of the feed.
class FeedManager {
  final DocumentManager _docMgr;
  final Engine _engine;
  final int _maxDocs;
  final DocumentRepository _docRepo;
  final ActiveDocumentDataRepository _activeRepo;

  FeedManager(this._docMgr, this._engine, this._maxDocs)
      : _docRepo = _docMgr.documentRepo,
        _activeRepo = _docMgr.activeRepo;

  /// Handle the given feed client event.
  ///
  /// Fails if [event] does not have a handler implemented.
  /// If [event] is [FeedRequested], returns the feed documents.
  /// Returns an empty list in all other cases.
  Future<List<Document>> handleFeedClientEvent(FeedClientEvent event) {
    return event.maybeWhen(
      feedRequested: () => restoreFeed(),
      nextFeedBatchRequested: () => nextFeedBatch().then((_) => []),
      feedDocumentsClosed: (ids) =>
          _docMgr.deactivateDocuments(ids).then((_) => []),
      orElse: throw UnimplementedError('handler not implemented for $event'),
    );
  }

  /// Generates the feed of active documents, ordered by their global rank.
  ///
  /// That is, documents are ordered by their timestamp, then local rank.
  Future<List<Document>> restoreFeed() => _docRepo.fetchAll().then(
        (docs) => docs
          ..retainWhere((doc) => doc.isActive)
          ..sort((doc1, doc2) {
            final ord = doc1.timestamp.compareTo(doc2.timestamp);
            return ord == 0
                ? doc1.personalizedRank.compareTo(doc2.personalizedRank)
                : ord;
          }),
      );

  /// Obtain the next batch of feed documents and persist to repositories.
  Future<void> nextFeedBatch() async {
    final feedDocs = _engine.getFeedDocuments(_maxDocs);

    await _docRepo.updateMany(feedDocs.keys);
    for (final feedDoc in feedDocs.entries) {
      final id = feedDoc.key.documentId;
      await _activeRepo.update(id, feedDoc.value);
    }
  }
}
