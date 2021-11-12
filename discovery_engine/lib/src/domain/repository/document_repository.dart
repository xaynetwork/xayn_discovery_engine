import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for accessing documents.
abstract class DocumentRepository {
  Future<Document?> fetchById(DocumentId id);
  Future<List<Document>> fetchAll();
  Future<void> update(Document doc);
}
