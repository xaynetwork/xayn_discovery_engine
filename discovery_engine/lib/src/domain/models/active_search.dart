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

import 'package:equatable/equatable.dart';
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/api/models/active_search.dart' as api;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show searchTypeId, searchByTypeId;

part 'active_search.g.dart';

/// [ActiveSearch] represents attributes of a performed search.
@HiveType(typeId: searchTypeId)
class ActiveSearch with EquatableMixin {
  @HiveField(0)
  final String searchTerm;
  @HiveField(1)
  final int requestedPageNb;
  @HiveField(2)
  final int pageSize;
  @HiveField(3, defaultValue: SearchBy.query)
  final SearchBy searchBy;

  const ActiveSearch({
    required this.searchTerm,
    required this.requestedPageNb,
    required this.pageSize,
    required this.searchBy,
  });

  ActiveSearch nextPageSearch() => ActiveSearch(
        searchTerm: searchTerm,
        requestedPageNb: requestedPageNb + 1,
        pageSize: pageSize,
        searchBy: searchBy,
      );

  api.ActiveSearch toApiRepr() =>
      api.ActiveSearch(searchBy: searchBy, searchTerm: searchTerm);

  @override
  List<Object?> get props => [searchTerm, requestedPageNb, pageSize, searchBy];
}

@HiveType(typeId: searchByTypeId)
enum SearchBy {
  @HiveField(0)
  query,
  @HiveField(1)
  topic,
}
