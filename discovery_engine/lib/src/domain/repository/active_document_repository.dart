import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for data relating to active documents.
abstract class ActiveDocumentRelatedDataRepository {
  Future<Uint8List?> smbertEmbeddingById(DocumentId id);
}
