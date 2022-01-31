// Copyright 2021 Xayn AG
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
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/duration.dart'
    show DurationFfi;

void main() {
  test('parsing written duration yields same result', () {
    const duration = Duration(seconds: 4949, microseconds: 5012);
    final place = ffi.alloc_uninitialized_duration_box();
    duration.writeNative(place);
    final res = DurationFfi.readNative(place);
    ffi.drop_duration_box(place);
    expect(res, equals(duration));
  });
}
