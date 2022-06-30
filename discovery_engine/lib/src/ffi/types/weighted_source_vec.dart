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

import 'dart:ffi';

import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/list.dart';
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source.dart';

final _adapter = ListFfiAdapter(
  alloc: ffi.alloc_uninitialized_weighted_source_slice,
  next: ffi.next_weighted_source,
  writeNative: (source, place) => source.writeNative(place),
  readNative: WeightedSourceFfi.readNative,
  getVecLen: ffi.get_weighted_source_vec_len,
  getVecBuffer: ffi.get_weighted_source_vec_buffer,
  writeNativeVec: ffi.init_weighted_source_vec_at,
);

extension WeightedSourceListFfi on List<SourceReacted> {
  static List<WeightedSourceFfi> readSlice(
    final Pointer<RustWeightedSource> ptr,
    final int len,
  ) =>
      _adapter.readSlice(ptr, len);

  Boxed<RustVecWeightedSource> allocVec() {
    final place = ffi.alloc_uninitialized_weighted_source_vec();
    final list = map(WeightedSourceFfi.fromSourceReacted).toList();
    _adapter.writeVec(list, place);
    return Boxed(place, ffi.drop_weighted_source_vec);
  }

  static List<WeightedSourceFfi> consumeBoxedVec(
    Boxed<RustVecWeightedSource> boxedVec,
  ) {
    final res = _adapter.readVec(boxedVec.ref);
    boxedVec.free();
    return res;
  }
}
