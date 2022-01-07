import 'package:hive/hive.dart' show TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show durationTypeId;

class DurationAdapter extends TypeAdapter<Duration> {
  @override
  final typeId = durationTypeId;

  @override
  Duration read(BinaryReader reader) {
    final value = reader.readInt();
    return Duration(microseconds: value);
  }

  @override
  void write(BinaryWriter writer, Duration obj) {
    writer.writeInt(obj.inMicroseconds);
  }
}
