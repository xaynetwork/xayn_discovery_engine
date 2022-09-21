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
import 'package:xayn_discovery_engine/discovery_engine.dart' show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart'
    show SourcePreference, PreferenceMode;
import 'package:xayn_discovery_engine/src/domain/repository/source_preference_repo.dart'
    show SourcePreferenceRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show sourcePreferenceBox;

class HiveSourcePreferenceRepository implements SourcePreferenceRepository {
  Box<SourcePreference> get box =>
      Hive.box<SourcePreference>(sourcePreferenceBox);

  @override
  Future<Set<Source>> getTrusted() async {
    return box.values
        .where((el) => el.mode == PreferenceMode.trusted)
        .map((el) => el.source)
        .toSet();
  }

  @override
  Future<Set<Source>> getExcluded() async {
    return box.values
        .where((el) => el.mode == PreferenceMode.excluded)
        .map((el) => el.source)
        .toSet();
  }

  @override
  Future<void> save(SourcePreference filter) async {
    await box.put(filter.source.value, filter);
  }

  @override
  Future<void> saveAll(Map<String, SourcePreference> filters) async {
    await box.putAll(filters);
  }

  @override
  Future<void> remove(Source source) async {
    await box.delete(source.value);
  }

  @override
  Future<void> clear() async {
    await box.clear();
    await box.flush();
  }

  @override
  Future<bool> isEmpty() => Future.value(box.isEmpty);
}
