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

import 'dart:ffi' show Pointer;

import 'package:xayn_discovery_engine/discovery_engine.dart' show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustMarket, RustVecMarket;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market.dart'
    show FeedMarketFfi;
import 'package:xayn_discovery_engine/src/ffi/types/list.dart';

final _adapter = ListFfiAdapter<FeedMarket, RustMarket, RustVecMarket>(
  alloc: ffi.alloc_uninitialized_market_slice,
  next: ffi.next_market,
  writeNative: (market, place) => market.writeNative(place),
  readNative: FeedMarketFfi.readNative,
  getVecLen: ffi.get_market_vec_len,
  getVecBuffer: ffi.get_market_vec_buffer,
  writeNativeVec: ffi.init_market_vec_at,
);

extension FeedMarketSliceFfi on List<FeedMarket> {
  /// Allocates a slice of markets containing all markets of this list.
  Pointer<RustMarket> createSlice() => _adapter.createSlice(this);

  /// Reads a `&[RustMarket]` returning a `List<FeedMarked>`.
  static List<FeedMarket> readSlice(
    final Pointer<RustMarket> ptr,
    final int len,
  ) =>
      _adapter.readSlice(ptr, len);

  Boxed<RustVecMarket> allocVec() {
    final place = ffi.alloc_uninitialized_market_vec();
    writeVec(place);
    return Boxed(place, ffi.drop_market_vec);
  }

  /// Writes a rust-`Vec<RustMarket>` to given place.
  void writeVec(
    final Pointer<RustVecMarket> place,
  ) =>
      _adapter.writeVec(this, place);

  /// Reads a rust-`&Vec<RustMarket>` returning a dart-`List<FeedMarket>`.
  static List<FeedMarket> readVec(
    final Pointer<RustVecMarket> vec,
  ) =>
      _adapter.readVec(vec);
}
