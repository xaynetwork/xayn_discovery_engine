import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

abstract class ActiveDocumentRepository {
  Future<int?> smbertEmbeddingById(DocumentId id); // FIXME
}
