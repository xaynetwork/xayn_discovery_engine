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

import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustHistoricDocument, RustVecHistoricDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/list.dart'
    show ListFfiAdapter;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uri.dart' show UriFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart'
    show DocumentIdFfi;

extension HistoricDocumentFfi on HistoricDocument {
  void writeNative(Pointer<RustHistoricDocument> place) {
    id.writeNative(ffi.historic_document_place_of_id(place));
    url.writeNative(ffi.historic_document_place_of_url(place));
    snippet.writeNative(ffi.historic_document_place_of_snippet(place));
    title.writeNative(ffi.historic_document_place_of_title(place));
  }

  @visibleForTesting
  Boxed<RustHistoricDocument> allocNative() {
    final place = ffi.alloc_uninitialized_historic_document();
    writeNative(place);
    return Boxed(place, ffi.drop_historic_document);
  }

  @visibleForTesting
  static HistoricDocument readNative(Pointer<RustHistoricDocument> doc) {
    return HistoricDocument(
      id: DocumentIdFfi.readNative(ffi.historic_document_place_of_id(doc)),
      url: UriFfi.readNative(ffi.historic_document_place_of_url(doc)),
      snippet:
          StringFfi.readNative(ffi.historic_document_place_of_snippet(doc)),
      title: StringFfi.readNative(ffi.historic_document_place_of_title(doc)),
    );
  }
}

final _listFfiAdapter = ListFfiAdapter(
  alloc: ffi.alloc_uninitialized_historic_document_slice,
  next: ffi.next_historic_document,
  writeNative: (doc, place) => doc.writeNative(place),
  readNative: HistoricDocumentFfi.readNative,
  getVecLen: ffi.get_historic_document_vec_len,
  getVecBuffer: ffi.get_historic_document_vec_buffer,
  writeNativeVec: ffi.init_historic_document_vec_at,
);

extension HistoricDocumentSliceFfi on List<HistoricDocument> {
  Boxed<RustVecHistoricDocument> allocNative() {
    final place = ffi.alloc_uninitialized_historic_document_vec();
    _listFfiAdapter.writeVec(this, place);
    return Boxed(place, ffi.drop_historic_document_vec);
  }

  @visibleForTesting
  static List<HistoricDocument> consumeNative(
    Boxed<RustVecHistoricDocument> boxed,
  ) {
    final res = _listFfiAdapter.readVec(boxed.ref);
    boxed.free();
    return res;
  }
}
