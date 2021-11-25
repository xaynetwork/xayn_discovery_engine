import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart';
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show activeDocumentDataTypeId;

part 'active_data.g.dart';

/// Additional data pertaining to active documents.
@HiveType(typeId: activeDocumentDataTypeId)
class ActiveDocumentData {
  @HiveField(0)
  final Uint8List smbertEmbedding;

  const ActiveDocumentData({
    required this.smbertEmbedding,
  });
}
