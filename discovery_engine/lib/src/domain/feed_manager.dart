import 'package:xayn_discovery_engine/src/api/events/client_events.dart';
import 'package:xayn_discovery_engine/src/domain/document_manager.dart'
    show DocumentManager;

/// Business logic concerning the management of the feed.
class FeedManager {
  final DocumentManager _documentMgr;

  FeedManager(this._documentMgr);

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
  Future<void> nextFeedBatch() async {} // TODO
}
