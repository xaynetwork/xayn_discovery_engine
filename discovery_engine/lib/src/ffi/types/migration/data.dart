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

import 'dart:ffi';

import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/list.dart';
import 'package:xayn_discovery_engine/src/ffi/types/migration/document.dart';
import 'package:xayn_discovery_engine/src/ffi/types/migration/search.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/ffi/types/source.dart';
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source_vec.dart';
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';

extension DartMigrationDataFfi on DartMigrationData {
  Boxed<RustDartMigrationData> allocNative() {
    final place = ffi.alloc_uninitialized_dart_migration_data();
    writeNative(place);
    return Boxed(place, ffi.drop_dart_migration_data);
  }

  void writeNative(final Pointer<RustDartMigrationData> place) {
    engineState
        .writeNative(ffi.dart_migration_data_place_of_engine_state(place));
    trustedSources
        .writeNative(ffi.dart_migration_data_place_of_trusted_sources(place));
    excludedSources
        .writeNative(ffi.dart_migration_data_place_of_excluded_sources(place));
    reactedSources
        .writeNative(ffi.dart_migration_data_place_of_reacted_sources(place));

    activeSearch.writeNative(ffi.dart_migration_data_place_of_search(place));

    final documentsWithData = documents
        .map(
          (document) => MigrationDocument(
            document,
            activeDocumentData[document.documentId],
          ),
        )
        .toList();

    _listAdapter.writeVec(
      documentsWithData,
      ffi.dart_migration_data_place_of_documents(place),
    );
  }
}

extension OptionDartMigrationDataFfi on DartMigrationData? {
  void writeNative(Pointer<RustOptionDartMigrationData> place) {
    final self = this;
    if (self == null) {
      ffi.init_option_dart_migration_data_none_at(place);
    } else {
      final data = self.allocNative();
      ffi.init_option_dart_migration_data_some_at(place, data.move());
    }
  }
}

final _listAdapter = ListFfiAdapter<MigrationDocument, RustMigrationDocument,
    RustVecMigrationDocument>(
  alloc: ffi.alloc_uninitialized_migration_document_slice,
  next: ffi.next_migration_document,
  writeNative: (doc, place) {
    doc.writeNative(place);
  },
  readNative: (_) => throw UnimplementedError(),
  getVecLen: (_) => throw UnimplementedError(),
  getVecBuffer: (_) => throw UnimplementedError(),
  writeNativeVec: ffi.init_migration_document_vec_at,
);
