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

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart';
import 'package:xayn_discovery_engine/src/ffi/types/date_time.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/news_resource.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart';
import 'package:xayn_discovery_engine/src/ffi/types/duration.dart';
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart';
import 'package:xayn_discovery_engine/src/ffi/types/list.dart';
import 'package:xayn_discovery_engine/src/ffi/types/migration_search.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/ffi/types/source.dart';
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart';
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

class MigrationDocument {
  final Document document;
  final ActiveDocumentData? activeData;

  MigrationDocument(this.document, this.activeData);

  void writeNative(Pointer<RustMigrationDocument> place) {
    document.documentId.writeNative(ffi.migration_document_place_of_id(place));
    document.stackId
        .writeNative(ffi.migration_document_place_of_stack_id(place));
    (activeData?.smbertEmbedding)
        .writeNative(ffi.migration_document_place_of_smbert_embedding(place));
    document.userReaction
        .writeNative(ffi.migration_document_place_of_reaction(place));
    document.resource
        .writeNative(ffi.migration_document_place_of_resource(place));
    ffi.init_migration_document_is_active_at(place, document.isActive ? 1 : 0);
    ffi.init_migration_document_is_searched_at(
      place,
      document.isSearched ? 1 : 0,
    );

    // ignore: deprecated_member_use_from_same_package
    document.batchIndex
        .writeNative(ffi.migration_document_place_of_batch_index(place));

    // ignore: deprecated_member_use_from_same_package
    document.timestamp
        .writeNative(ffi.migration_document_place_of_timestamp(place));

    (activeData?.viewTime[DocumentViewMode.web])
        .writeNative(ffi.migration_document_place_of_web_view_time(place));
    (activeData?.viewTime[DocumentViewMode.reader])
        .writeNative(ffi.migration_document_place_of_reader_view_time(place));
    (activeData?.viewTime[DocumentViewMode.story])
        .writeNative(ffi.migration_document_place_of_story_view_time(place));
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
