import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/document_repo.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show documentBox;

/// Hive repository implementation of [DocumentRepository].
class HiveDocumentRepository implements DocumentRepository {
  Box<Document> get box => Hive.box<Document>(documentBox);

  @override
  Future<Document?> fetchById(DocumentId id) async {
    return box.get(id.toString());
  }

  @override
  Future<List<Document>> fetchAll() async => box.values.toList();

  @override
  Future<void> update(Document doc) async {
    final key = doc.documentId.toString();
    await box.put(key, doc);
  }
}
