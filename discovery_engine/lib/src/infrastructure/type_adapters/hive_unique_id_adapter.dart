// Copyright 2022 Xayn AG
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

import 'package:hive/hive.dart' show TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show UniqueId, DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show documentIdTypeId, stackIdTypeId;

abstract class _UniqueIdAdapter<T extends UniqueId> extends TypeAdapter<T> {
  @override
  void write(BinaryWriter writer, T obj) {
    final bytes = obj.value.buffer.asUint8List();
    writer.writeByteList(bytes);
  }

  @override
  T read(BinaryReader reader) {
    final bytes = reader.readByteList();
    switch (typeId) {
      case documentIdTypeId:
        return DocumentId.fromBytes(bytes) as T;
      case stackIdTypeId:
        return StackId.fromBytes(bytes) as T;
      default:
        throw ArgumentError.value('unrecognized "typeId"');
    }
  }
}

class DocumentIdAdapter extends _UniqueIdAdapter<DocumentId> {
  @override
  final typeId = documentIdTypeId;
}

class StackIdAdapter extends _UniqueIdAdapter<StackId> {
  @override
  final typeId = stackIdTypeId;
}
