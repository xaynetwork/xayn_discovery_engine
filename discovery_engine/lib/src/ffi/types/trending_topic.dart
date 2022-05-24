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
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart';
import 'package:xayn_discovery_engine/src/domain/models/trending_topic.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart';
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uri.dart' show UriFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart';

class TrendingTopicFfi with EquatableMixin {
  final DocumentId id;
  final Embedding smbertEmbedding;
  final String name;
  final String query;
  final Uri? image;

  TrendingTopicFfi({
    required this.id,
    required this.smbertEmbedding,
    required this.name,
    required this.query,
    required this.image,
  });

  @override
  List<Object?> get props => [id, smbertEmbedding, name, query, image];

  factory TrendingTopicFfi.readNative(final Pointer<RustTrendingTopic> place) {
    return TrendingTopicFfi(
      id: DocumentIdFfi.readNative(ffi.trending_topic_place_of_id(place)),
      smbertEmbedding: EmbeddingFfi.readNative(
        ffi.trending_topic_place_of_smbert_embedding(place),
      ),
      name: StringFfi.readNative(ffi.trending_topic_place_of_name(place)),
      query: StringFfi.readNative(ffi.trending_topic_place_of_query(place)),
      image: UriFfi.readNativeOption(ffi.trending_topic_place_of_image(place)),
    );
  }

  void writeNative(final Pointer<RustTrendingTopic> place) {
    id.writeNative(ffi.trending_topic_place_of_id(place));
    smbertEmbedding
        .writeNative(ffi.trending_topic_place_of_smbert_embedding(place));
    name.writeNative(ffi.trending_topic_place_of_name(place));
    query.writeNative(ffi.trending_topic_place_of_query(place));
    UriFfi.writeNativeOption(image, ffi.trending_topic_place_of_image(place));
  }

  TrendingTopic toTrendingTopic() => TrendingTopic(
        name: name,
        query: query,
        image: image,
      );
}
