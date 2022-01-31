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

import 'dart:convert' show utf8;
import 'dart:ffi' show Pointer, Uint8Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustString;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension StringFfi on String {
  void writeNative(final Pointer<RustString> place) {
    final utf8Bytes = utf8.encode(this);
    final len = utf8Bytes.length;
    ffi.init_string_at(place, len).asTypedList(len).setAll(0, utf8Bytes);
    ffi.set_string_len(place, len);
  }

  static String readNative(
    final Pointer<RustString> place,
  ) {
    final len = ffi.get_string_len(place);
    final data = ffi.get_string_buffer(place).asTypedList(len);
    return utf8.decode(data);
  }
}
