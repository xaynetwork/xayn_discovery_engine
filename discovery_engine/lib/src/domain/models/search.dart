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

import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart';
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show searchTypeId;

part 'search.freezed.dart';
part 'search.g.dart';

/// [Search] is representing attributes of a performed search query.
@freezed
class Search with _$Search {
  @HiveType(typeId: searchTypeId)
  const factory Search({
    @HiveField(0) required String queryTerm,
    @HiveField(1) required int requestedPageNb,
    @HiveField(2) required int pageSize,
    @HiveField(3) required FeedMarket market,
  }) = _Search;

  factory Search.fromJson(Map<String, Object?> json) => _$SearchFromJson(json);
}
