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
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show searchTypeId, searchByTypeId;

part 'active_search.g.dart';

/// [ActiveSearch] represents attributes of a performed search.
@HiveType(typeId: searchTypeId)
// TODO: after DB migration
// - rename `ActiveSearch` -> `Search` to match the rust side
// - remove api/ActiveSearch & expose domain/ActiveSearch instead
// - reorder arguments in the public events to have 1. `by` & 2. `term` to match the logical order of the rust side
class ActiveSearch with EquatableMixin {
  // TODO: rename `searchBy` -> `by` & `searchTerm` -> `term` to reduce redundancy & to match the rust side after DB migration
  @HiveField(3, defaultValue: SearchBy.query)
  final SearchBy searchBy;
  @HiveField(0)
  final String searchTerm;

  // TODO: remove these fields after DB migration
  @HiveField(1)
  int requestedPageNb;
  @HiveField(2)
  final int pageSize;

  ActiveSearch({
    required this.searchBy,
    required this.searchTerm,
    required this.requestedPageNb,
    required this.pageSize,
  });

  @override
  List<Object?> get props => [searchBy, searchTerm, requestedPageNb, pageSize];
}

@HiveType(typeId: searchByTypeId)
enum SearchBy {
  @HiveField(0)
  query,
  @HiveField(1)
  topic,
}
