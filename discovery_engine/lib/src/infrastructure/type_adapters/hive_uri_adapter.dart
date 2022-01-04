import 'package:hive/hive.dart' show TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show uriTypeId;

class UriAdapter extends TypeAdapter<Uri> {
  @override
  final typeId = uriTypeId;

  @override
  Uri read(BinaryReader reader) {
    final uriStr = reader.readString();
    return Uri.parse(uriStr);
  }

  @override
  void write(BinaryWriter writer, Uri obj) {
    writer.writeString(obj.toString());
  }
}
