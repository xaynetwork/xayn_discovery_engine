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

import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/search.dart'
    show Search;
import 'package:xayn_discovery_engine/src/domain/repository/search_repo.dart'
    show SearchRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show searchBox;

/// Hive repository implementation of [SearchRepository].
class HiveSearchRepository implements SearchRepository {
  static const stateKey = 0;

  Box<Search> get box => Hive.box<Search>(searchBox);

  @override
  Future<Search?> getCurrent() async => box.get(stateKey);

  @override
  Future<void> clear() async {
    await box.clear();
  }

  @override
  Future<void> save(Search data) => box.put(stateKey, data);
}
