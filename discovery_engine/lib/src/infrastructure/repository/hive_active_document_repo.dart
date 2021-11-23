import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repository.dart'
    show ActiveDocumentRelatedDataRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show activeDocumentDataBox;

class HiveActiveDocumentRelatedDataRepository
    implements ActiveDocumentRelatedDataRepository {
  Box<ActiveDocumentData> get box =>
      Hive.box<ActiveDocumentData>(activeDocumentDataBox);

  @override
  Future<Uint8List?> smbertEmbeddingById(DocumentId id) async {
    var activeDoc = box.get(id.toString());
    return activeDoc?.smbertEmbedding;
  }
}
