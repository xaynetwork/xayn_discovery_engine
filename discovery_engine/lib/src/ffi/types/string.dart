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

import 'dart:convert' show utf8;
import 'dart:ffi' show Pointer, Uint8, Uint8Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustString;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension StringFfi on String {
  void writeNative(final Pointer<RustString> place) {
    final str = BoxedStr.create(this);
    ffi.init_string_at(place, str.start, str.len);
  }

  static String readNative(
    final Pointer<RustString> place,
  ) =>
      BoxedStr.fromRawParts(
        ffi.get_string_buffer(place),
        ffi.get_string_len(place),
      ).readNative();
}

class BoxedStr {
  final Pointer<Uint8> start;
  final int len;

  BoxedStr.fromRawParts(this.start, this.len);

  /// Creates a `Box<str>` based on given dart string.
  factory BoxedStr.create(String string) {
    final utf8Bytes = utf8.encode(string);
    final len = utf8Bytes.length;
    final start = ffi.alloc_uninitialized_bytes(len);
    start.asTypedList(len).setAll(0, utf8Bytes);
    return BoxedStr.fromRawParts(start, len);
  }

  /// Call to free allocated memory if not moved to dart.
  void free() {
    ffi.drop_bytes(this.start, this.len);
  }

  String readNative() => utf8.decode(start.asTypedList(len));
}
