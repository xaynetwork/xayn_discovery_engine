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
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    as domain;
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show SearchBy;

// Re-export public parts of `domain/` to avoid juggling multiple `active_search.dart`
// imports in the same file.
export 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show SearchBy;

part 'active_search.freezed.dart';
part 'active_search.g.dart';

/// [ActiveSearch] represents attributes of a performed search.
@freezed
class ActiveSearch with _$ActiveSearch {
  const factory ActiveSearch({
    required SearchBy searchBy,
    required String searchTerm,
  }) = _ActiveSearch;

  factory ActiveSearch.fromJson(Map<String, Object?> json) =>
      _$ActiveSearchFromJson(json);
}

@protected
extension ActiveSearchApiConversion on domain.ActiveSearch {
  ActiveSearch toApiRepr() =>
      ActiveSearch(searchBy: searchBy, searchTerm: searchTerm);
}
