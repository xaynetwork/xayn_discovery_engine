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

import 'dart:typed_data';

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart'
    show Embedding1Ffi;

void main() {
  test('parsing written empty embeddings works', () {
    final embedding = Float32List(0);
    final place = ffi.alloc_uninitialized_embedding1_box();
    embedding.writeNative(place);
    final res = Embedding1Ffi.readNative(place);
    ffi.drop_embedding1_box(place);
    expect(res, equals(embedding));
  });

  test('parsing written embeddings yields same result', () {
    final embedding =
        Float32List.fromList([18.4, 6.9, 13.2, 7.8945, 8.2, 0.3, 7.8, 9.479]);
    final place = ffi.alloc_uninitialized_embedding1_box();
    embedding.writeNative(place);
    final res = Embedding1Ffi.readNative(place);
    ffi.drop_embedding1_box(place);
    expect(res, equals(embedding));
  });
}
