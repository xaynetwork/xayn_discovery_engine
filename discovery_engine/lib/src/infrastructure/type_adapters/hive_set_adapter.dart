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

import 'package:hive/hive.dart' show BinaryReader, BinaryWriter, TypeAdapter;

abstract class SetAdapter<T> extends TypeAdapter<Set<T>> {
  TypeAdapter<T> get delegateAdapter;

  @override
  Set<T> read(BinaryReader reader) {
    final length = reader.readUint32();
    return Iterable.generate(length, (_) => delegateAdapter.read(reader))
        .toSet();
  }

  @override
  void write(BinaryWriter writer, Set<T> obj) {
    writer.writeUint32(obj.length);
    for (final item in obj) {
      delegateAdapter.write(writer, item);
    }
  }
}
