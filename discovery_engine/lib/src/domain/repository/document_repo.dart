import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Document repository interface.
abstract class DocumentRepository {
  /// Fetch document by id.
  Future<Document?> fetchById(DocumentId id);

  /// Fetch documents by ids.
  ///
  /// Any id that does not identify a document is ignored.
  Future<List<Document>> fetchByIds(Set<DocumentId> ids);

  /// Fetch all documents.
  Future<List<Document>> fetchAll();

  /// Update with the given document.
  Future<void> update(Document doc);

  /// Update with the given documents.
  ///
  /// If [docs] contains multiple documents with the same id, the last
  /// occurrence with that id will overwrite previous occurrences.
  Future<void> updateMany(Iterable<Document> docs);
}
