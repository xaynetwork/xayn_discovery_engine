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
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;

void main() {
  test('parsing written empty string works', () {
    const string = '';
    final place = ffi.alloc_uninitialized_string_box();
    string.writeNative(place);
    final res = StringFfi.readNative(place);
    ffi.drop_string_box(place);
    expect(res, equals(string));
  });

  test('parsing written string yields same result', () {
    const string = 'a&+KLA)&+fjw)&+f';
    final place = ffi.alloc_uninitialized_string_box();
    string.writeNative(place);
    final res = StringFfi.readNative(place);
    ffi.drop_string_box(place);
    expect(res, equals(string));
  });
}
