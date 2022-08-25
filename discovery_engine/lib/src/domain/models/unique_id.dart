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

/// [UniqueId] represents base for unique identifiers for other models like
/// [StackId] or [DocumentId].
abstract class UniqueId with EquatableMixin {
  final UnmodifiableUint8ListView value;

  UniqueId() : value = _generateId();

  UniqueId.fromBytes(Uint8List bytes) : value = _validateId(bytes);

  UniqueId.fromJson(Map<String, Object> json)
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
class DocumentId extends UniqueId {
  DocumentId() : super();
  DocumentId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  DocumentId.fromJson(Map<String, Object> json) : super.fromJson(json);
}

/// Unique identifier of a stack that the [Document] belongs to.
class StackId extends UniqueId {
  static const _names = {
    '77cf9280-bb93-4158-b660-8732927e0dcc': 'Exploration',
    '1ce442c8-8a96-433e-91db-c0bee37e5a83': 'BreakingNews',
    '311dc7eb-5fc7-4aa4-8232-e119f7e80e76': 'PersonalizedNews',
    'd0f699d8-60d2-4008-b3a1-df1cffc4b8a3': 'TrustedNews',
  };

  String get name => _names[toString()] ?? 'UnknownStack';

  StackId() : super();
  StackId.fromBytes(Uint8List bytes) : super.fromBytes(bytes);
  StackId.fromJson(Map<String, Object> json) : super.fromJson(json);
  StackId.nil() : super.fromBytes(Uint8List(16));
}
