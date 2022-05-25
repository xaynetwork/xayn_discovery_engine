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

import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/list.dart';
import 'package:xayn_discovery_engine/src/ffi/types/trending_topic.dart';

final _adapter =
    ListFfiAdapter<TrendingTopicFfi, RustTrendingTopic, RustVecTrendingTopic>(
  alloc: ffi.alloc_uninitialized_trending_topic_slice,
  next: ffi.next_trending_topic,
  writeNative: (trendingTopic, place) => trendingTopic.writeNative(place),
  readNative: TrendingTopicFfi.readNative,
  getVecLen: ffi.get_trending_topic_vec_len,
  getVecBuffer: ffi.get_trending_topic_vec_buffer,
  writeNativeVec: ffi.init_trending_topic_vec_at,
);

extension TrendingTopicSliceFfi on List<TrendingTopicFfi> {
  /// Allocates a slice containing all trending topics of this list.
  Pointer<RustTrendingTopic> createSlice() => _adapter.createSlice(this);

  static List<TrendingTopicFfi> readSlice(
    final Pointer<RustTrendingTopic> ptr,
    final int len,
  ) =>
      _adapter.readSlice(ptr, len);

  /// Writes a rust-`Vec<RustTrendingTopic>` to given place.
  void writeVec(
    final Pointer<RustVecTrendingTopic> place,
  ) =>
      _adapter.writeVec(this, place);

  /// Reads a rust-`&Vec<RustTrendingTopic>` returning a dart-`List<TrendingTopic>`.
  static List<TrendingTopicFfi> readVec(
    final Pointer<RustVecTrendingTopic> vec,
  ) =>
      _adapter.readVec(vec);

  /// Consumes a `Box<Vec<TrendingTopic>>` returned from rust.
  ///
  /// The additional indirection is necessary due to dart
  /// not handling custom non-boxed, non-primitive return
  /// types well.
  static List<TrendingTopicFfi> consumeBoxedVector(
    Pointer<RustVecTrendingTopic> boxedVec,
  ) {
    final res = readVec(boxedVec);
    ffi.drop_trending_topic_vec(boxedVec);
    return res;
  }

  List<TrendingTopic> toTrendingTopicList() =>
      asMap().entries.map((e) => e.value.toTrendingTopic()).toList();
}
