import 'dart:typed_data' show Uint8List;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Additional data pertaining to active documents.
class ActiveDocumentData {
  final DocumentId id;
  final Uint8List smbertEmbedding;

  const ActiveDocumentData._({
    required this.id,
    required this.smbertEmbedding,
  });
}
