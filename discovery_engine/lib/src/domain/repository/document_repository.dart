import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Document repository interface.
abstract class DocumentRepository {
  /// Fetch document by id.
  Future<Document?> fetchById(DocumentId id);

  /// Fetch all documents.
  Future<List<Document>> fetchAll();

  /// Update with the given document.
  Future<void> update(Document doc);
}
