import 'package:xayn_discovery_engine/src/api/events/client_events/document_events.dart'
    show DocumentClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback, DocumentStatus;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;

class DocumentManager {
  final DocumentRepository _repo;

  DocumentManager(this._repo);

  void handle(DocumentClientEvent evt) {
    evt.when(
      documentStatusChanged: (DocumentId id, DocumentStatus status) =>
          print('baz'),
      documentClosed: (DocumentId id) => print('bar'),
      documentFeedbackChanged: (DocumentId id, DocumentFeedback feedback) =>
          handleFeedbackChanged(id, feedback),
    );
  }

  /// Handle [DocumentClientEvent.documentFeedbackChanged].
  Future<void> handleFeedbackChanged(
    DocumentId id,
    DocumentFeedback feedback,
  ) async {
    final doc = await _repo.fetchById(id);
    final updatedDoc = doc?.setFeedback(feedback);
    await _repo.update(updatedDoc!);
  }
}
