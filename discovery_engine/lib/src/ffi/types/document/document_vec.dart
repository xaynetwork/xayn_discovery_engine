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

import 'dart:ffi' show Pointer;

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDocument, RustDocumentVec;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show DocumentFfi;

extension DocumentSliceFfi on List<DocumentFfi> {
  /// Allocates a slice of documents containing all documents of this list.
  Pointer<RustDocument> createSlice() {
    final slice = ffi.alloc_uninitialized_document_slice(length);
    fold<Pointer<RustDocument>>(slice, (nextElement, document) {
      document.writeNative(nextElement);
      return ffi.next_document(nextElement);
    });
    return slice;
  }

  static List<DocumentFfi> readSlice(
    final Pointer<RustDocument> ptr,
    final int len,
  ) {
    final out = <DocumentFfi>[];
    Iterable<int>.generate(len).fold<Pointer<RustDocument>>(ptr,
        (nextElement, _) {
      out.add(DocumentFfi.readNative(nextElement));
      return ffi.next_document(nextElement);
    });
    return out;
  }

  /// Consumes a `Box<Vec<Document>>` returned from rust.
  ///
  /// The additional indirection is necessary due to dart
  /// not handling custom non-boxed, non-primitive return
  /// types well.
  static List<DocumentFfi> consumeBoxedVector(
    Pointer<RustDocumentVec> boxedVec,
  ) {
    final len = ffi.get_document_vec_len(boxedVec);
    final slice = ffi.get_document_vec_buffer(boxedVec);
    final res = readSlice(slice, len);
    ffi.drop_document_vec(boxedVec);
    return res;
  }
}
