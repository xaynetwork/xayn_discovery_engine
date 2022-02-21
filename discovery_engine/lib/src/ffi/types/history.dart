import 'dart:ffi' show Pointer;

import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/domain/models/history.dart'
    show HistoricDocument;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustHistoricDocument;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
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
