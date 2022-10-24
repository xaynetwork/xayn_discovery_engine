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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/time_spent.dart'
    show TimeSpentFfi;

void main() {
  test('reading written time spent instance yields same result', () {
    final timeSpent = TimeSpent(
      id: DocumentId(),
      viewTime: const Duration(days: 2),
      viewMode: DocumentViewMode.story,
    );
    final place = ffi.alloc_uninitialized_time_spent();
    timeSpent.writeNative(place);
    final res = TimeSpentFfi.readNative(place);
    ffi.drop_time_spent(place);
    expect(res, equals(timeSpent));
  });
}
