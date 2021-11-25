import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for ids of documents whose status has changed since the
/// previous call of the feedback loop.
abstract class ChangedDocumentRepository {
  /// Fetch ids of all changed documents.
  Future<List<DocumentId>> fetchAll();

  /// Add the id of a changed document.
  Future<void> add(DocumentId id);

  /// Clear the repository.
  Future<void> removeAll();
}
