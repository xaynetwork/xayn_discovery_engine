import 'package:hive/hive.dart' show TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentIdTypeId;

class DocumentIdAdapter extends TypeAdapter<DocumentId> {
  @override
  final typeId = documentIdTypeId;

  @override
  DocumentId read(BinaryReader reader) {
    final bytes = reader.readByteList();
    return DocumentId.fromBytes(bytes);
  }

  @override
  void write(BinaryWriter writer, DocumentId obj) {
    final bytes = obj.value.buffer.asUint8List();
    writer.writeByteList(bytes);
  }
}
