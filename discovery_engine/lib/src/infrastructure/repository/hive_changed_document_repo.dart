import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repository.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show changedDocumentIdBox;

class HiveChangedDocumentRepository implements ChangedDocumentRepository {
  Box<DocumentId> get box => Hive.box<DocumentId>(changedDocumentIdBox);

  @override
  Future<List<DocumentId>> fetchAllIds() async => box.values.toList();

  @override
  Future<void> add(DocumentId id) async {
    await box.add(id);
  }

  @override
  Future<void> removeAll() async {
    await box.clear();
  }
}
