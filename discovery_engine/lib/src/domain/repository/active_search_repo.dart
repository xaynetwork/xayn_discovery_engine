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

import 'package:xayn_discovery_engine/src/domain/ai_state_holder.dart';
import 'package:xayn_discovery_engine/src/domain/models/active_search.dart'
    show ActiveSearch;

/// Repository interface for currently performed [ActiveSearch].
abstract class ActiveSearchRepository implements AIStateHolder {
  /// Get current search.
  Future<ActiveSearch?> getCurrent();

  /// Update current search.
  Future<void> save(ActiveSearch data);

  /// Remove current search.
  Future<void> clear();
}
