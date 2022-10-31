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

import 'package:equatable/equatable.dart';
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uri.dart' show UriFfi;

class TrendingTopicFfi with EquatableMixin {
  final String name;
  final String query;
  final Uri? image;

  TrendingTopicFfi({
    required this.name,
    required this.query,
    required this.image,
  });

  @override
  List<Object?> get props => [name, query, image];

  factory TrendingTopicFfi.readNative(final Pointer<RustTrendingTopic> place) {
    return TrendingTopicFfi(
      name: StringFfi.readNative(ffi.trending_topic_place_of_name(place)),
      query: StringFfi.readNative(ffi.trending_topic_place_of_query(place)),
      image: UriFfi.readNativeOption(ffi.trending_topic_place_of_image(place)),
    );
  }

  void writeNative(final Pointer<RustTrendingTopic> place) {
    name.writeNative(ffi.trending_topic_place_of_name(place));
    query.writeNative(ffi.trending_topic_place_of_query(place));
    UriFfi.writeNativeOption(image, ffi.trending_topic_place_of_image(place));
  }

  TrendingTopic toTrendingTopic() =>
      TrendingTopic(name: name, query: query, image: image);
}
