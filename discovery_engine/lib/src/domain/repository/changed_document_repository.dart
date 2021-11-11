import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

abstract class ChangedDocumentRepository {
  Future<List<DocumentId>> fetchAllIds();
  Future<void> add(Document doc);
  Future<void> removeAll();
}
