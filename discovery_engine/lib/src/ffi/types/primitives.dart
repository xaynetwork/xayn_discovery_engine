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

import 'dart:ffi' show FloatPointer, Pointer, Uint8Pointer;
import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustOptionF32, RustVecU8;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;

class PrimitivesFfi {
  static void writeNativeOptionF32(
    double? value,
    Pointer<RustOptionF32> place,
  ) {
    if (value == null) {
      ffi.init_none_f32_at(place);
    } else {
      ffi.init_some_f32_at(place, value);
    }
  }

  static double? readNativeOptionF32(Pointer<RustOptionF32> option) {
    final ptr = ffi.get_option_f32_some(option);
    if (ptr.address == 0) {
      return null;
    } else {
      return ptr.value;
    }
  }
}

extension Uint8ListFfi on Uint8List {
  Boxed<RustVecU8> allocNative() {
    final ptr = ffi.alloc_uninitialized_bytes(length);
    ptr.asTypedList(length).setAll(0, this);
    final vec = ffi.alloc_vec_u8(ptr, length);
    return Boxed(vec, ffi.drop_vec_u8);
  }

  static Uint8List readNative(Pointer<RustVecU8> vec) {
    final len = ffi.get_vec_u8_len(vec);
    final buffer = ffi.get_vec_u8_buffer(vec);
    return Uint8List.fromList(buffer.asTypedList(len));
  }
}
