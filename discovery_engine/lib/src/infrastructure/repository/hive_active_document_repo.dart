import 'dart:typed_data' show Uint8List;
// import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/active_document_repository.dart'
    show ActiveDocumentRelatedDataRepository;

class HiveActiveDocumentRelatedDataRepository
    implements ActiveDocumentRelatedDataRepository {
  @override
  Future<Uint8List?> smbertEmbeddingById(DocumentId id) async {
    // TODO
  }
}
