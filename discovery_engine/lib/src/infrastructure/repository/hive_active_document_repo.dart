import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repo.dart'
    show ActiveDocumentDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox;

/// Hive repository implementation of [ActiveDocumentDataRepository].
class HiveActiveDocumentDataRepository implements ActiveDocumentDataRepository {
  Box<ActiveDocumentData> get box =>
      Hive.box<ActiveDocumentData>(activeDocumentDataBox);

  @override
  Future<ActiveDocumentData?> fetchById(DocumentId id) async =>
      box.get(id.toString());

  @override
  Future<Uint8List?> smbertEmbeddingById(DocumentId id) async =>
      box.get(id.toString())?.smbertEmbedding;

  @override
  Future<void> update(DocumentId id, ActiveDocumentData data) async {
    await box.put(id.toString(), data);
  }

  @override
  Future<void> removeByIds(Iterable<DocumentId> ids) async {
    await box.deleteAll(ids);
  }
}
