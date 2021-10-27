import 'dart:typed_data' show UnmodifiableUint8ListView, Uint8List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:uuid/uuid.dart' show Uuid;

/// [UniqueId] represents base for unique identifiers for other models like
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
  bool? get stringify => true;
}

/// Unique identifier of a [Document].
class DocumentId extends _UniqueId {
  DocumentId() : super();
  DocumentId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
}

/// Unique identifier of a search.
class SearchId extends _UniqueId {
  SearchId() : super();
  SearchId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
}
