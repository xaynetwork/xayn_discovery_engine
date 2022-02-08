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
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart';

extension DocumentSliceFfi on List<Document> {
  /// Allocates a slice of documents containing all documents of this list.
  Pointer<RustDocument> createSlice() {
    final slice = ffi.alloc_uninitialized_document_slice(length);
    fold<Pointer<RustDocument>>(slice, (nextElement, document) {
      document.writeTo(nextElement);
      return ffi.next_document(nextElement);
    });
    return slice;
  }

  static List<Document> readSlice(
    final Pointer<RustDocument> slice,
    final int len,
  ) {
    final out = <Document>[];
    Iterable<int>.generate(len).fold<Pointer<RustDocument>>(slice,
        (nextElement, _) {
      out.add(Document.readFrom(nextElement));
      return ffi.next_document(nextElement);
    });
    return out;
  }

  /// Consumes a `Box<Vec<Document>>` returned from rust.
  ///
  /// The additional indirection is necessary due to dart
  /// not handling custom non-boxed, non-primitive return
  /// types well.
  static List<Document> consumeBoxedVector(
    Pointer<RustDocumentVec> boxedVec,
  ) {
    final len = ffi.get_document_vec_len(boxedVec);
    final slice = ffi.get_document_vec_buffer(boxedVec);
    final res = readSlice(slice, len);
    ffi.drop_document_vec(boxedVec);
    return res;
  }
}
