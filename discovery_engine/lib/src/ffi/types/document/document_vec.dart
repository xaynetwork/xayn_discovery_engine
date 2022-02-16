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

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show DocumentWithActiveData;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustDocument, RustVecDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/document/document.dart'
    show DocumentFfi;
import 'package:xayn_discovery_engine/src/ffi/types/list.dart'
    show ListFfiAdapter;

final _adapter = ListFfiAdapter<DocumentFfi, RustDocument, RustVecDocument>(
  alloc: ffi.alloc_uninitialized_document_slice,
  next: ffi.next_document,
  writeNative: (document, place) => document.writeNative(place),
  readNative: (place) => DocumentFfi.readNative(place),
  getVecLen: ffi.get_document_vec_len,
  getVecBuffer: ffi.get_document_vec_buffer,
  writeNativeVec: ffi.init_document_vec_at,
);

extension DocumentSliceFfi on List<DocumentFfi> {
  /// Allocates a slice of documents containing all documents of this list.
  Pointer<RustDocument> createSlice() => _adapter.createSlice(this);

  static List<DocumentFfi> readSlice(
    final Pointer<RustDocument> ptr,
    final int len,
  ) =>
      _adapter.readSlice(ptr, len);

  /// Writes a rust-`Vec<RustDocument>` to given place.
  void writeVec(
    final Pointer<RustVecDocument> place,
  ) =>
      _adapter.writeVec(this, place);

  /// Reads a rust-`&Vec<RustDocument>` returning a dart-`List<Document>`.
  static List<DocumentFfi> readVec(
    final Pointer<RustVecDocument> vec,
  ) =>
      _adapter.readVec(vec);

  /// Consumes a `Box<Vec<Document>>` returned from rust.
  ///
  /// The additional indirection is necessary due to dart
  /// not handling custom non-boxed, non-primitive return
  /// types well.
  static List<DocumentFfi> consumeBoxedVector(
    Pointer<RustVecDocument> boxedVec,
  ) {
    final res = readVec(boxedVec);
    ffi.drop_document_vec(boxedVec);
    return res;
  }

  List<DocumentWithActiveData> toDocumentListWithActiveData() => asMap()
      .entries
      .map(
        (e) => DocumentWithActiveData(
          e.value.toDocument(batchIndex: e.key),
          e.value.toActiveDocumentData(),
        ),
      )
      .toList();
}
