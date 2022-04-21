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
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show searchTypeId, searchByTypeId;

part 'active_search.freezed.dart';
part 'active_search.g.dart';

/// [ActiveSearch] is representing attributes of a performed search query.
@freezed
class ActiveSearch with _$ActiveSearch {
  @HiveType(typeId: searchTypeId)
  const factory ActiveSearch({
    @HiveField(0) required String queryTerm,
    @HiveField(1) required int requestedPageNb,
    @HiveField(2) required int pageSize,
    @HiveField(3) required SearchBy searchBy,
  }) = _ActiveSearch;

  factory ActiveSearch.fromJson(Map<String, Object?> json) =>
      _$ActiveSearchFromJson(json);
}

@HiveType(typeId: searchByTypeId)
enum SearchBy {
  @HiveField(0) query,
  @HiveField(1) topic,
}
