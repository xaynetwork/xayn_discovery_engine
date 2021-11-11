import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

abstract class DocumentRepository {
  Future<Document?> fetchById(DocumentId id);
  Future<List<Document>> fetchAll();
  Future<void> update(); // FIXME type signature
}
