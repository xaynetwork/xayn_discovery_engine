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

import 'dart:typed_data' show Uint8List;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show Uint8ListFfi;

void main() {
  test('reading written bytes', () {
    final bytes = Uint8List.fromList([1, 4, 3, 2]);
    final nativeBytes = bytes.allocNative();
    final res = Uint8ListFfi.readNative(nativeBytes.ref);
    nativeBytes.free();
    expect(res, equals(bytes));
  });
}
