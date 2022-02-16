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

import 'dart:typed_data' show Float32List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:hive/hive.dart' show BinaryReader, BinaryWriter, TypeAdapter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show embeddingAdapter;

/// 1-Dimensional Embedding
///
/// Values are stored in native byte order in hive.
class Embedding with EquatableMixin {
  final Float32List values;

  Embedding(this.values);

  @override
  List<Object?> get props => [values];
}

class EmbeddingAdapter extends TypeAdapter<Embedding> {
  @override
  int get typeId => embeddingAdapter;

  @override
  Embedding read(BinaryReader reader) =>
      Embedding(reader.readByteList().buffer.asFloat32List());

  @override
  void write(BinaryWriter writer, Embedding obj) {
    writer.writeByteList(obj.values.buffer.asUint8List());
  }
}
