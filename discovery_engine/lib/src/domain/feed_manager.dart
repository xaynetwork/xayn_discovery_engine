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
  final DocumentManager _documentMgr;
  final Engine _engine;
  final DocumentRepository _docRepo;
  final ActiveDocumentDataRepository _activeRepo;

  FeedManager(this._documentMgr, this._engine)
      : _docRepo = _documentMgr.documentRepo,
        _activeRepo = _documentMgr.activeRepo;

  /// Handle the given feed client event.
  ///
  /// Fails if [evt] does not have a handler implemented.
  Future<void> handleFeedClientEvent(FeedClientEvent evt) async {
    await evt.maybeWhen(
      feedRequested: () => restoreFeed(),
      nextFeedBatchRequested: () => nextFeedBatch(),
      feedDocumentsClosed: (ids) => _documentMgr.deactivateDocuments(ids),
      orElse: throw UnimplementedError('handler not implemented for $evt'),
    );
  }

  Future<void> restoreFeed() async {} // TODO

  Future<void> nextFeedBatch() async {
    const maxDocs = 10; // FIXME hard coded for now

    final feedDocs = _engine.getFeedDocuments(maxDocs);

    await _docRepo.updateMany(feedDocs.keys);
    for (final feedDoc in feedDocs.entries) {
      final id = feedDoc.key.documentId;
      await _activeRepo.update(id, feedDoc.value);
    }
  }
}
