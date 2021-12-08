import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for ids of documents whose status has changed since the
/// previous call of the feedback loop.
abstract class ChangedDocumentRepository {
  /// Fetch all the document ids.
  Future<List<DocumentId>> fetchAll();

  /// Add the id of a changed document.
  ///
  /// This has no effect if [id] has already been added.
  Future<void> add(DocumentId id);

  /// Clear the repository.
  Future<void> removeAll();

  /// Remove the given document ids.
  Future<void> removeMany(Iterable<DocumentId> ids);
}
