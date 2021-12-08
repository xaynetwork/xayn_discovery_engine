import 'package:xayn_discovery_engine/src/api/events/client_events.dart'
    show ClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentFeedback, DocumentViewMode;
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

  /// Handle the given client event.
  ///
  /// Throws if the event does not have a handler implemented.
  void handleClientEvent(ClientEvent evt) {
    evt.maybeWhen(
      documentFeedbackChanged: (id, fdbk) => updateDocumentFeedback(id, fdbk),
      feedDocumentsClosed: (ids) => deactivateDocuments(ids),
      documentTimeLogged: (id, mode, sec) =>
          addActiveDocumentTime(id, mode, sec),
      orElse: throw UnimplementedError('handler not implemented for $evt'),
    );
  }

  /// Update feedback for the given document.
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

  /// Add additional viewing time for the given active document.
  Future<void> addActiveDocumentTime(
    DocumentId id,
    DocumentViewMode mode,
    int sec,
  ) async {
    final activeData = await _activeRepo.fetchById(id);
    if (activeData != null) {
      activeData.addViewTime(mode, Duration(seconds: sec));
      await _activeRepo.update(id, activeData);
    }
  }
}
