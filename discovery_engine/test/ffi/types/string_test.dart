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
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show OptionStringFfi, StringFfi;

void main() {
  test('reading written empty string works', () {
    const string = '';
    final place = ffi.alloc_uninitialized_string();
    string.writeNative(place);
    final res = StringFfi.readNative(place);
    ffi.drop_string(place);
    expect(res, equals(string));
  });

  test('reading written string yields same result', () {
    const string = 'a&+KLA)&+fjw)&+f';
    final place = ffi.alloc_uninitialized_string();
    string.writeNative(place);
    final res = StringFfi.readNative(place);
    ffi.drop_string(place);
    expect(res, equals(string));
  });

  test('reading written some string yields same result', () {
    // ignore: unnecessary_cast
    const String? string = 'a&+KLA)&+fjw)&+f' as String?;
    final place = ffi.alloc_uninitialized_option_string();
    string.writeNative(place);
    final res = OptionStringFfi.readNative(place);
    ffi.drop_option_string(place);
    expect(res, equals(string));
  });

  test('reading written none string yields same result', () {
    const String? string = null;
    final place = ffi.alloc_uninitialized_option_string();
    string.writeNative(place);
    final res = OptionStringFfi.readNative(place);
    ffi.drop_option_string(place);
    expect(res, isNull);
  });
}
