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
    show RustMarket, RustMarketVec;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market.dart'
    show FeedMarketFfi;

extension FeedMarketSliceFfi on List<FeedMarket> {
  /// Allocates a slice of markets containing all markets of this list.
  Pointer<RustMarket> createSlice() {
    final slice = ffi.alloc_uninitialized_market_slice(length);
    fold<Pointer<RustMarket>>(slice, (nextElement, market) {
      market.writeNative(nextElement);
      return ffi.next_market(nextElement);
    });
    return slice;
  }

  static List<FeedMarket> readSlice(
    final Pointer<RustMarket> ptr,
    final int len,
  ) {
    final out = <FeedMarket>[];
    Iterable<int>.generate(len).fold<Pointer<RustMarket>>(ptr,
        (nextElement, _) {
      out.add(FeedMarketFfi.readNative(nextElement));
      return ffi.next_market(nextElement);
    });
    return out;
  }

  /// Consumes a `Box<Vec<Market>>` returned from rust.
  static List<FeedMarket> consumeBoxedVector(
    Pointer<RustMarketVec> boxedVec,
  ) {
    final len = ffi.get_market_vec_len(boxedVec);
    final slice = ffi.get_market_vec_buffer(boxedVec);
    final res = readSlice(slice, len);
    ffi.drop_market_vec(boxedVec);
    return res;
  }
}
