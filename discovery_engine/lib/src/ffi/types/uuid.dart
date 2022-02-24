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

import 'dart:ffi' show Pointer, Uint8Pointer;
import 'dart:typed_data' show Uint8List;

import 'package:uuid/uuid.dart' show Uuid;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustUuid;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show BoxedStr;

extension DocumentIdFfi on DocumentId {
  void writeNative(final Pointer<RustUuid> place) {
    _writeUuid(place, value);
  }

  static DocumentId readNative(final Pointer<RustUuid> place) {
    return DocumentId.fromBytes(_readUuid(place));
  }
}

extension StackIdFfi on StackId {
  void writeNative(final Pointer<RustUuid> place) {
    _writeUuid(place, value);
  }

  static StackId readNative(final Pointer<RustUuid> place) {
    return StackId.fromBytes(_readUuid(place));
  }
}

void _writeUuid(final Pointer<RustUuid> place, final Uint8List id) {
  final str = BoxedStr.create(Uuid.unparse(id));
  try {
    final ok = ffi.init_uuid_from_string_at(place, str.ptr, str.len);
    if (ok != 1) {
      throw ArgumentError("can't parse uuid");
    }
  } finally {
    str.free();
  }
}

Uint8List _readUuid(final Pointer<RustUuid> uuid) {
  final strPtr = ffi.get_uuid_as_string36(uuid);
  final boxed = BoxedStr.fromRawParts(strPtr, 36);
  try {
    return Uuid.parseAsByteList(boxed.readNative());
  } finally {
    boxed.free();
  }
}

// void _writeUuid(final Pointer<RustUuid> uuidPlace, final Uint8List id) {
//   if (id.length != 16) {
//     throw ArgumentError('uuid must have exactly 16 bytes');
//   }
//   ffi.init_uuid_at(
//     uuidPlace,
//     id[0],
//     id[1],
//     id[2],
//     id[3],
//     id[4],
//     id[5],
//     id[6],
//     id[7],
//     id[8],
//     id[9],
//     id[10],
//     id[11],
//     id[12],
//     id[13],
//     id[14],
//     id[15],
//   );
// }

// Uint8List _readUuid(final Pointer<RustUuid> uuidPlace) {
//   final beginOfData = ffi.get_uuid_bytes(uuidPlace);
//   final view = beginOfData.asTypedList(16);
//   return Uint8List.fromList(view);
// }
