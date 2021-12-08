import 'dart:typed_data' show Uint8List;

import 'package:hive/hive.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show activeDocumentDataTypeId;

part 'active_data.g.dart';

/// Additional data pertaining to active documents.
@HiveType(typeId: activeDocumentDataTypeId)
class ActiveDocumentData {
  @HiveField(0)
  final Uint8List smbertEmbedding;
  @HiveField(1)
  final Map<DocumentViewMode, Duration> viewTime;

  ActiveDocumentData(this.smbertEmbedding) : viewTime = {};

  /// Add a time interval to the running total for the given view mode.
  void addViewTime(DocumentViewMode mode, Duration time) {
    viewTime.update(mode, (total) => total + time, ifAbsent: () => time);
  }

  /// Get the time spent in the given view mode.
  Duration getViewTime(DocumentViewMode mode) {
    return viewTime[mode] ?? Duration.zero;
  }
}
