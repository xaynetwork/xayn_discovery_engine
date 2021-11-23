import 'dart:typed_data' show UnmodifiableUint8ListView, Uint8List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:uuid/uuid.dart' show Uuid;

/// [_UniqueId] represents base for unique identifiers for other models like
/// [SearchId] or [DocumentId].
abstract class _UniqueId with EquatableMixin {
  final UnmodifiableUint8ListView value;

  _UniqueId() : value = _generateId();

  _UniqueId.fromBytes(Uint8List bytes) : value = _validateId(bytes);

  static UnmodifiableUint8ListView _generateId() {
    final id = Uuid().v4();
    final bytes = Uuid.parseAsByteList(id);
    return UnmodifiableUint8ListView(bytes);
  }

  static UnmodifiableUint8ListView _validateId(Uint8List bytes) {
    Uuid.isValidOrThrow(fromByteList: bytes);
    return UnmodifiableUint8ListView(bytes);
  }

  @override
  List<Object?> get props => [value];

  @override
  String toString() => Uuid.unparse(value);

  static Uint8List bytesFromJson(Map<String, dynamic> json) =>
      Uint8List.fromList((json['value'] as List).cast<int>());

  Map<String, dynamic> toJson() => <String, dynamic>{
        'value': value.buffer.asUint8List(),
      };
}

/// Unique identifier of a Document.
class DocumentId extends _UniqueId {
  DocumentId() : super();
  DocumentId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);

  factory DocumentId.fromJson(Map<String, dynamic> json) =>
      DocumentId.fromBytes(_UniqueId.bytesFromJson(json));
}

/// Unique identifier of a search.
class SearchId extends _UniqueId {
  SearchId() : super();
  SearchId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);

  factory SearchId.fromJson(Map<String, dynamic> json) =>
      SearchId.fromBytes(_UniqueId.bytesFromJson(json));
}
