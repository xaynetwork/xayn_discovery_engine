import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show FeedClientEvent;
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;
import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
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
  Future<void> handleFeedClientEvent(FeedClientEvent event) async {
    await event.maybeWhen(
      feedRequested: () => restoreFeed(),
      nextFeedBatchRequested: () => nextFeedBatch(),
      feedDocumentsClosed: (ids) => _docMgr.deactivateDocuments(ids),
      orElse: throw UnimplementedError('handler not implemented for $event'),
    );
  }

  Future<void> restoreFeed() async {} // TODO once timestamps addded to Document

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
