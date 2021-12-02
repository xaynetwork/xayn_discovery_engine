import 'package:xayn_discovery_engine/src/api/events/client_events/document_events.dart'
    show DocumentClientEvent;
import 'package:xayn_discovery_engine/src/api/events/client_events/feed_events.dart'
    show FeedClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;

class DocumentManager {
  final DocumentRepository _documentRepo;
  final ActiveDocumentDataRepository _activeRepo;
  final ChangedDocumentRepository _changedRepo;

  DocumentManager(this._documentRepo, this._activeRepo, this._changedRepo);

  void handleDocumentEvent(DocumentClientEvent evt) {
    evt.when(
      documentStatusChanged: (id, status) => print('baz'),
      documentClosed: (id) => print('bar'),
      documentFeedbackChanged: (id, feedback) =>
          updateDocumentFeedback(id, feedback),
    );
  }

  /// Update the feedback for the given document.
  ///
  /// Has no effect if `id` does not identify an active document.
  Future<void> updateDocumentFeedback(
    DocumentId id,
    DocumentFeedback feedback,
  ) async {
    final doc = await _documentRepo.fetchById(id);
    if (doc != null && doc.isActive) {
      final updatedDoc = doc.setFeedback(feedback);
      await _documentRepo.update(updatedDoc);
    }
  }

  void handleFeedEvent(FeedClientEvent evt) {
    evt.when(
      feedDocumentsClosed: (ids) => deactivateDocuments(ids),
      nextFeedBatchRequested: () => print('bar'),
      feedRequested: () => print('baz'),
    );
  }

  /// Deactivate the given documents.
  Future<void> deactivateDocuments(Set<DocumentId> ids) async {
    await _activeRepo.removeByIds(ids);
    await _changedRepo.removeMany(ids);

    for (final id in ids) {
      var doc = await _documentRepo.fetchById(id);
      doc = doc?.setInactive();
      if (doc != null) await _documentRepo.update(doc);
    }
  }
}
