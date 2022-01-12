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
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show uriTypeId;

class UriAdapter extends TypeAdapter<Uri> {
  @override
  final typeId = uriTypeId;

  @override
  Uri read(BinaryReader reader) {
    final uriStr = reader.readString();
    return Uri.parse(uriStr);
  }

  @override
  void write(BinaryWriter writer, Uri obj) {
    writer.writeString(obj.toString());
  }
}
