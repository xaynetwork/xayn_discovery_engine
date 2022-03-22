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
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show setStringTypeId;

class SetStringAdapter extends TypeAdapter<Set<String>> {
  @override
  int get typeId => setStringTypeId;

  @override
  Set<String> read(BinaryReader reader) {
    final length = reader.readUint32();
    return Iterable.generate(length, (_) => reader.readString()).toSet();
  }

  @override
  void write(BinaryWriter writer, Set<String> obj) {
    writer.writeUint32(obj.length);
    for (final item in obj) {
      writer.writeString(item);
    }
  }
}
