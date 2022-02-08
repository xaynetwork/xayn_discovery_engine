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
import 'package:xayn_discovery_engine/src/ffi/types/uri.dart' show UriFfi;

void main() {
  test('reading written uri works', () {
    final uri = Uri.parse('https://foo.example/bar');
    final place = ffi.alloc_uninitialized_url();
    uri.writeNative(place);
    final res = UriFfi.readNative(place);
    ffi.drop_url(place);
    expect(uri, equals(res));
  });

  test('reading written Option::Some uri works', () {
    final uri = Uri.parse('https://foo.example/bar');
    final place = ffi.alloc_uninitialized_option_url();
    UriFfi.writeNativeOption(uri, place);
    final res = UriFfi.readNativeOption(place);
    ffi.drop_option_url(place);
    expect(res, equals(uri));
  });

  test('reading written Option::None uri works', () {
    final place = ffi.alloc_uninitialized_option_url();
    UriFfi.writeNativeOption(null, place);
    final res = UriFfi.readNativeOption(place);
    ffi.drop_option_url(place);
    expect(res, isNull);
  });
}
