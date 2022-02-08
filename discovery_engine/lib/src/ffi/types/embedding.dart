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

import 'dart:ffi' show Pointer, FloatPointer;
import 'dart:typed_data' show Float32List;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustEmbedding;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;

extension EmbeddingFfi on Float32List {
  void writeNative(
    final Pointer<RustEmbedding> place,
  ) {
    final len = length;
    final buffer = ffi.alloc_uninitialized_f32_slice(len);
    buffer.asTypedList(len).setAll(0, this);
    ffi.init_embedding_at(place, buffer, len);
  }

  static Float32List readNative(
    final Pointer<RustEmbedding> place,
  ) {
    final len = ffi.get_embedding_buffer_len(place);
    final data = ffi.get_embedding_buffer(place).asTypedList(len);
    return Float32List.fromList(data);
  }
}
