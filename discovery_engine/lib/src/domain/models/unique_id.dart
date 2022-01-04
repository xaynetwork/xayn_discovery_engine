import 'dart:typed_data' show UnmodifiableUint8ListView, Uint8List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:hive/hive.dart' show TypeAdapter, BinaryReader, BinaryWriter;
import 'package:uuid/uuid.dart' show Uuid;

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentIdTypeId;

/// [_UniqueId] represents base for unique identifiers for other models like
/// [SearchId] or [DocumentId].
abstract class _UniqueId with EquatableMixin {
  final UnmodifiableUint8ListView value;

  _UniqueId() : value = _generateId();

  _UniqueId.fromBytes(Uint8List bytes) : value = _validateId(bytes);

  _UniqueId.fromJson(Map<String, Object> json)
      : value = _validateId(_bytesFromJson(json));

  static UnmodifiableUint8ListView _generateId() {
    final id = const Uuid().v4();
    final bytes = Uuid.parseAsByteList(id);
    return UnmodifiableUint8ListView(bytes);
  }

  static UnmodifiableUint8ListView _validateId(Uint8List bytes) {
    Uuid.isValidOrThrow(fromByteList: bytes);
    return UnmodifiableUint8ListView(bytes);
  }

  static Uint8List _bytesFromJson(Map<String, Object> json) {
    return Uint8List.fromList((json['value'] as List).cast<int>());
  }

  @override
  List<Object?> get props => [value];

  @override
  String toString() => Uuid.unparse(value);

  Map<String, Object> toJson() => <String, Object>{
        'value': value.buffer.asUint8List(),
      };
}

/// Unique identifier of a [Document].
class DocumentId extends _UniqueId {
  DocumentId() : super();
  DocumentId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  DocumentId.fromJson(Map<String, Object> json) : super.fromJson(json);
}

/// Unique identifier of a search.
class SearchId extends _UniqueId {
  SearchId() : super();
  SearchId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  SearchId.fromJson(Map<String, Object> json) : super.fromJson(json);
}

// Can be generated automatically
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
