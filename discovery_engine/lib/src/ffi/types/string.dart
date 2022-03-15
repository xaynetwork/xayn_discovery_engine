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
import 'dart:ffi' show nullptr, Pointer, Uint8, Uint8Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustOptionString, RustString;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show checkFfiUsize;

extension StringFfi on String {
  void writeNative(final Pointer<RustString> place) {
    final str = BoxedStr.create(this);
    ffi.init_string_at(place, str.ptr, str.len);
  }

  static String readNative(
    final Pointer<RustString> place,
  ) =>
      BoxedStr.fromRawParts(
        ffi.get_string_buffer(place),
        ffi.get_string_len(place),
      ).readNative();

  Boxed<RustString> allocNative() {
    final place = ffi.alloc_uninitialized_string();
    writeNative(place);
    return Boxed(place, ffi.drop_string);
  }
}

extension OptionStringFfi on String? {
  void writeNative(final Pointer<RustOptionString> place) {
    if (this == null) {
      ffi.init_option_string_none_at(place);
    } else {
      final str = BoxedStr.create(this!);
      ffi.init_option_string_some_at(place, str.ptr, str.len);
    }
  }

  static String? readNative(final Pointer<RustOptionString> place) {
    final str = ffi.get_option_string_some(place);
    if (str == nullptr) {
      return null;
    } else {
      return StringFfi.readNative(str);
    }
  }
}

class BoxedStr {
  final Pointer<Uint8> ptr;
  final int len;

  BoxedStr.fromRawParts(this.ptr, this.len) {
    checkFfiUsize(len, 'BoxedStr.len');
  }

  /// Creates a `Box<str>` based on given dart string.
  factory BoxedStr.create(String string) {
    final utf8Bytes = utf8.encode(string);
    final len = utf8Bytes.length;
    checkFfiUsize(len, 'String.len');
    final ptr = ffi.alloc_uninitialized_bytes(len);
    ptr.asTypedList(len).setAll(0, utf8Bytes);
    return BoxedStr.fromRawParts(ptr, len);
  }

  /// Call to free allocated memory if not moved to dart.
  void free() {
    ffi.drop_bytes(ptr, len);
  }

  String readNative() {
    final data = ptr.asTypedList(len);
    return utf8.decode(data);
  }
}
