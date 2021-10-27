import 'dart:typed_data';

/// [UniqueId] represent base for unique identifier for other models like
/// "search" or [Document].
abstract class _UniqueId {
  final UnmodifiableUint8ListView value;

  _UniqueId(this.value);
}

/// Unique identifier of a [Document].
class DocumentId extends _UniqueId {
  DocumentId._(UnmodifiableUint8ListView value) : super(value);

  factory DocumentId() {
    // TODO: this is just temporary, it requires a real implementation
    final id = UnmodifiableUint8ListView(Uint8List(0));
    return DocumentId._(id);
  }
}

/// Unique identifier of a search.
class SearchId extends _UniqueId {
  SearchId._(UnmodifiableUint8ListView value) : super(value);
}
