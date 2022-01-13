// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
