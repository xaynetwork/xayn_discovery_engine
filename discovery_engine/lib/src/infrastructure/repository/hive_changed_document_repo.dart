import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/changed_document_repo.dart'
    show ChangedDocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show changedDocumentIdBox;

/// Hive repository implementation of [ChangedDocumentRepository].
class HiveChangedDocumentRepository implements ChangedDocumentRepository {
  Box<Uint8List> get box => Hive.box<Uint8List>(changedDocumentIdBox);

  @override
  Future<List<DocumentId>> fetchAll() async =>
      box.values.map((bytes) => DocumentId.fromBytes(bytes)).toList();

  @override
  Future<void> add(DocumentId id) async {
    await box.put(id.toString(), id.value);
  }

  @override
  Future<void> removeAll() async {
    await box.clear();
  }
}
