import 'dart:typed_data';

/// [UniqueId] represent base for unique identifier for other models like
/// "search" or [Document].
abstract class UniqueId {
  final Uint8List value;

  UniqueId(this.value);
}

/// Unique identifier of a [Document].
class DocumentId extends UniqueId {
  DocumentId._(Uint8List value) : super(value);

  factory DocumentId() {
    // TODO: this is just temporary, it requires a real implementation
    final id = Uint8List(0);
    return DocumentId._(id);
  }
}

/// Unique identifier of a search.
class SearchId extends UniqueId {
  SearchId._(Uint8List value) : super(value);
}
