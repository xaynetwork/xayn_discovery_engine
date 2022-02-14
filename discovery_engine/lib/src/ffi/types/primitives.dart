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

import 'dart:ffi' show FloatPointer, Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustOptionF32;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

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
