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
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/date_time.dart'
    show NaiveDateTimeFfi;

void readWriteTest(DateTime dateTime) {
  final place = ffi.alloc_uninitialized_naive_date_time();
  dateTime.writeNative(place);
  final res = NaiveDateTimeFfi.readNative(place);
  ffi.drop_naive_date_time(place);
  expect(res, equals(dateTime));
}

void main() {
  test('reading written naive date time yields same result', () {
    readWriteTest(DateTime.now());
  });

  test('reading written absurd large naive date time yields same result', () {
    // At some point larger then this it will fail.
    const hugeTimeWeStillSupport = 200000 * 365 * 24 * 60 * 60 * 1000000;
    final dateTime =
        DateTime.fromMicrosecondsSinceEpoch(hugeTimeWeStillSupport);
    readWriteTest(dateTime);
  });

  test('reading written pre-epoch time yields same result', () {
    final dateTime = DateTime.parse('0500-06-21 18:17:12');
    readWriteTest(dateTime);
  });
}
