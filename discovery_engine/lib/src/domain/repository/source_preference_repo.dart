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

import 'package:xayn_discovery_engine/discovery_engine.dart' show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_preference.dart'
    show SourcePreference;

abstract class SourcePreferenceRepository {
  Future<Set<Source>> getTrusted();

  Future<Set<Source>> getExcluded();

  Future<void> save(SourcePreference filter);

  Future<void> saveAll(Map<String, SourcePreference> filters);

  Future<void> remove(Source source);

  Future<void> clear();

  /// Checks if the repository is semantically empty.
  Future<bool> isEmpty();
}
