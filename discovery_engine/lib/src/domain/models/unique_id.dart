import 'dart:typed_data' show UnmodifiableUint8ListView, Uint8List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:uuid/uuid.dart' show Uuid;

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;

/// [_UniqueId] represents base for unique identifiers for other models like
/// [SearchId] or [DocumentId].
abstract class _UniqueId with EquatableMixin {
  final UnmodifiableUint8ListView value;

  _UniqueId() : value = _generateId();

  _UniqueId.fromBytes(Uint8List bytes) : value = _validateId(bytes);

  _UniqueId.fromJson(Map<String, dynamic> json)
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

  static Uint8List _bytesFromJson(Map<String, dynamic> json) {
    return Uint8List.fromList((json['value'] as List).cast<int>());
  }

  @override
  List<Object?> get props => [value];

  @override
  String toString() => Uuid.unparse(value);

  Map<String, dynamic> toJson() => <String, dynamic>{
        'value': value.buffer.asUint8List(),
      };
}

/// Unique identifier of a [Document].
class DocumentId extends _UniqueId {
  DocumentId() : super();
  DocumentId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  DocumentId.fromJson(Map<String, dynamic> json) : super.fromJson(json);
}

/// Unique identifier of a search.
class SearchId extends _UniqueId {
  SearchId() : super();
  SearchId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  SearchId.fromJson(Map<String, dynamic> json) : super.fromJson(json);
}
