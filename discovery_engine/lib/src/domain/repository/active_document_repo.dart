import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for additional data relating to active documents.
abstract class ActiveDocumentDataRepository {
  /// Fetch the SMBert embedding associated with the given document.
  Future<Uint8List?> smbertEmbeddingById(DocumentId id);

  /// Update data associated with the given active document.
  Future<void> update(DocumentId id, ActiveDocumentData data);
}
