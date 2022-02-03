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
  /// Allocates a slice of documents containing all document of this list.
  ///
  /// We expect the length of this list to be passed in.
  ///
  //Note: The len arguments makes sure the len you use with the slice and
  //      the len you create it with are the same, alternatively returning
  //      a `Pair` or custom type could be done.
  Pointer<RustDocument> createSlice(final int len) {
    if (len != length) {
      throw ArgumentError.value(len, 'len', 'len must match length');
    }
    final slice = ffi.alloc_uninitialized_document_slice(len);
    var nextElement = slice;
    for (final document in this) {
      document.writeTo(nextElement);
      nextElement = ffi.next_document(nextElement);
    }
    return slice;
  }

  static List<Document> readSlice(
    final Pointer<RustDocument> slice,
    final int len,
  ) {
    final out = <Document>[];
    for (var c = 0, next = slice;
        c < len;
        c++, next = ffi.next_document(next)) {
      out.add(Document.readFrom(next));
    }
    return out;
  }

  /// Consumes a `Box<Vec<Document>>` returned form rust.
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
