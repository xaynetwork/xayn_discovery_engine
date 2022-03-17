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
import 'package:xayn_discovery_engine/src/domain/repository/excluded_sources_repo.dart'
    show ExcludedSourcesRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show excludedSourcesBox;

/// Hive repository implementation of [ExcludedSourcesRepository].
class HiveExcludedSourcesRepository implements ExcludedSourcesRepository {
  static const stateKey = 0;

  Box<Set<String>> get box => Hive.box<Set<String>>(excludedSourcesBox);

  @override
  Future<Set<String>> getAll() async {
    return box.get(stateKey) ?? <String>{};
  }

  @override
  Future<void> save(Set<String> sources) async {
    await box.put(stateKey, sources);
  }
}
