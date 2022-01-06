import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentViewModeTypeId;

part 'view_mode.g.dart';

/// Document viewer mode.
@HiveType(typeId: documentViewModeTypeId)
enum DocumentViewMode {
  @HiveField(0)
  story,
  @HiveField(1)
  reader,
  @HiveField(2)
  web,
}
