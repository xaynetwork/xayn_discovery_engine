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

import 'dart:ffi' show Pointer, Uint32Pointer;

import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode, DocumentViewModeIntConversion;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustViewMode1;

extension DocumentViewModeFfi on DocumentViewMode {
  void writeNative(final Pointer<RustViewMode1> place) {
    place.value = toIntRepr();
  }

  static DocumentViewMode readNative(
    final Pointer<RustViewMode1> place,
  ) =>
      DocumentViewModeIntConversion.fromIntRepr(place.value);
}
